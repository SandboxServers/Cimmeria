//! Port of ServerEd's ScriptOptimizer / ScriptNodeCompiler to Rust.
//!
//! Takes a ScriptFile (visual graph) + ScriptDefinitions (node templates) and
//! produces a Python class that the Cimmeria game server can load.

use anyhow::{bail, Result};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use super::definitions::{NodeTemplate, ScriptDefinitions};
use super::xml_format::ScriptFile;

// ---------------------------------------------------------------------------
// Compiler-internal graph types (mirrors ScriptNode / ScriptConnection in C++)
// ---------------------------------------------------------------------------

const MAX_OPTIMIZATION_ITERATIONS: usize = 10;
const MAX_INLINE_LENGTH: usize = 4;

// ---------------------------------------------------------------------------
// Node + Connection flags as plain constants
// ---------------------------------------------------------------------------

mod node_flags {
    pub const FORCE: u32 = 0x0001;
    pub const ELIMINATE: u32 = 0x0002;
    pub const TRIGGER: u32 = 0x0004;
    pub const INIT_SCRIPT: u32 = 0x0008;
    pub const TEARDOWN_SCRIPT: u32 = 0x0010;
    pub const PERSIST_SCRIPT: u32 = 0x0020;
    pub const RESTORE_SCRIPT: u32 = 0x0040;
    pub const DONT_INLINE: u32 = 0x0080;
    pub const CALLED: u32 = 0x0100;
    pub const COMPILED: u32 = 0x0200;
    pub const INLINED: u32 = 0x0400;
    pub const REQUIRED: u32 = 0x0800;
    pub const NAMED_METHOD: u32 = 0x1000;
    pub const PRE_INIT_SCRIPT: u32 = 0x2000;
    pub const STATIC_INIT_SCRIPT: u32 = 0x4000;
    pub const STATIC_TEARDOWN_SCRIPT: u32 = 0x8000;

    pub const DYNAMIC: u32 = ELIMINATE | DONT_INLINE | COMPILED | INLINED;
}

mod conn_flags {
    pub const TRIGGER: u32 = 0x01;
    pub const CONNECTED: u32 = 0x02;
    pub const READ: u32 = 0x04;
    pub const WRITE: u32 = 0x08;
    pub const ACCESS: u32 = 0x10;
    pub const PROPAGATE: u32 = 0x20;
    pub const ELIMINATE: u32 = 0x40;
    pub const PERSISTENT: u32 = 0x80;

    pub const DYNAMIC: u32 = TRIGGER | READ | WRITE | ACCESS | PROPAGATE | ELIMINATE;
}

use conn_flags as CF;
use node_flags as NF;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScriptNodeType {
    InputPort,
    OutputPort,
    Script,
}

#[derive(Debug)]
struct CompNode {
    block_id: u32,
    name: String,
    node_type: ScriptNodeType,
    flags: u32,
    script: String,
    compiled_script: String,
    /// Indices into the CompGraph::connections vec
    out_links: Vec<usize>,
    in_links: Vec<usize>,
}

impl CompNode {
    fn is_alive(&self) -> bool {
        self.flags & NF::ELIMINATE == 0
    }

    fn is_compiled(&self) -> bool {
        self.flags & NF::COMPILED != 0
    }

    fn should_emit_function(&self) -> bool {
        (self.flags & (NF::CALLED | NF::DONT_INLINE)) != 0 || (self.flags & NF::INLINED) == 0
    }
}

#[derive(Debug)]
struct CompConnection {
    source: usize, // index into nodes
    target: usize,
    flags: u32,
}

impl CompConnection {
    fn is_alive(&self) -> bool {
        self.flags & CF::ELIMINATE == 0
    }
}

/// Holds the block-level data for a graph node (editor block).
#[derive(Debug)]
struct Block {
    id: u32,
    name: String,
    debug: bool,
    node_indices: HashMap<String, usize>, // port/script name -> node index
    properties: HashMap<String, String>,
}

impl Block {
    fn port(&self, name: &str) -> Option<usize> {
        self.node_indices.get(name).copied()
    }

    fn property(&self, name: &str) -> Option<&String> {
        self.properties.get(name)
    }
}

/// The full compilation graph -- owns all nodes and connections.
struct CompGraph {
    nodes: Vec<CompNode>,
    connections: Vec<CompConnection>,
    blocks: HashMap<u32, Block>,
    ordered_nodes: Vec<usize>,
    /// Template ref names for each block, so we can look up imports
    block_templates: HashMap<u32, String>,
}

impl CompGraph {
    fn new() -> Self {
        Self {
            nodes: Vec::new(),
            connections: Vec::new(),
            blocks: HashMap::new(),
            ordered_nodes: Vec::new(),
            block_templates: HashMap::new(),
        }
    }

    fn add_node(&mut self, block_id: u32, node: CompNode) -> usize {
        let name = node.name.clone();
        let idx = self.nodes.len();
        self.nodes.push(node);
        if let Some(block) = self.blocks.get_mut(&block_id) {
            block.node_indices.insert(name, idx);
        }
        idx
    }

    fn add_connection(&mut self, source: usize, target: usize, flags: u32) -> usize {
        let idx = self.connections.len();
        self.connections.push(CompConnection {
            source,
            target,
            flags,
        });
        self.nodes[source].out_links.push(idx);
        self.nodes[target].in_links.push(idx);
        idx
    }

    /// Add or update a dependency link (mirrors ScriptOptimizer::addDependency)
    fn add_dependency(&mut self, node_idx: usize, dep_idx: usize, flags: u32) {
        if node_idx == dep_idx {
            return;
        }
        // Check for existing connection
        for &ci in &self.nodes[node_idx].out_links {
            if self.connections[ci].target == dep_idx {
                self.connections[ci].flags |= flags;
                return;
            }
        }
        // Create new
        self.add_connection(node_idx, dep_idx, flags);
    }

    // --- Link query helpers (match C++ hasOutboundLinksAll / hasInboundLinksAny) ---

    fn has_outbound_links_all(
        &self,
        node_idx: usize,
        conn_flags: u32,
        node_flags: u32,
    ) -> bool {
        for &ci in &self.nodes[node_idx].out_links {
            let c = &self.connections[ci];
            if c.is_alive()
                && (conn_flags == 0 || (c.flags & conn_flags) == conn_flags)
                && (node_flags == 0 || (self.nodes[c.target].flags & node_flags) == node_flags)
            {
                return true;
            }
        }
        false
    }

    fn has_inbound_links_all(
        &self,
        node_idx: usize,
        conn_flags: u32,
        _node_flags: u32,
    ) -> bool {
        for &ci in &self.nodes[node_idx].in_links {
            let c = &self.connections[ci];
            if c.is_alive()
                && (conn_flags == 0 || (c.flags & conn_flags) == conn_flags)
            {
                return true;
            }
        }
        false
    }

    #[allow(dead_code)]
    fn has_outbound_links_any(
        &self,
        node_idx: usize,
        conn_flags: u32,
        node_flags: u32,
    ) -> bool {
        for &ci in &self.nodes[node_idx].out_links {
            let c = &self.connections[ci];
            if c.is_alive()
                && (conn_flags == 0 || (c.flags & conn_flags) != 0)
                && (node_flags == 0 || (self.nodes[c.target].flags & node_flags) != 0)
            {
                return true;
            }
        }
        false
    }

    fn has_inbound_links_any(
        &self,
        node_idx: usize,
        conn_flags: u32,
        _node_flags: u32,
    ) -> bool {
        for &ci in &self.nodes[node_idx].in_links {
            let c = &self.connections[ci];
            if c.is_alive() && (conn_flags == 0 || (c.flags & conn_flags) != 0) {
                return true;
            }
        }
        false
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Compile a script graph to Python source code.
pub fn compile_script(script: &ScriptFile, defs: &ScriptDefinitions) -> Result<String> {
    let template_map: HashMap<&str, &NodeTemplate> =
        defs.nodes.iter().map(|n| (n.ref_name.as_str(), n)).collect();

    let mut graph = build_graph(script, defs, &template_map)?;
    optimize_and_compile(&mut graph)?;
    emit_python(&graph, script, defs, &template_map)
}

/// Compile a script and write the output Python file to the correct location.
/// Returns the output path.
pub fn compile_to_file(
    script: &ScriptFile,
    defs: &ScriptDefinitions,
    repo_root: &Path,
) -> Result<PathBuf> {
    let python_code = compile_script(script, defs)?;

    let subdir = match script.script_type.as_str() {
        "Mission" => "missions",
        "Effect" => "effects",
        "Level" => "spaces",
        _ => bail!("Unknown script type: {}", script.script_type),
    };

    // Module "General.test" -> "General/test.py"
    let module_path = script.module.replace('.', "/");
    let out_path = repo_root
        .join("python/cell")
        .join(subdir)
        .join(format!("{}.py", module_path));

    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&out_path, python_code)?;

    Ok(out_path)
}

// ---------------------------------------------------------------------------
// Graph construction (mirrors ScriptOptimizer::addBlock / addBlockConnections)
// ---------------------------------------------------------------------------

fn build_graph(
    script: &ScriptFile,
    _defs: &ScriptDefinitions,
    template_map: &HashMap<&str, &NodeTemplate>,
) -> Result<CompGraph> {
    let mut graph = CompGraph::new();

    // Pass 1: Add all blocks (nodes from the script file)
    for snode in &script.nodes {
        let tpl = match template_map.get(snode.ref_name.as_str()) {
            Some(t) => *t,
            None => bail!("Unknown node template reference: '{}'", snode.ref_name),
        };

        // Build property map: start with template defaults, override with script values
        let mut props: HashMap<String, String> = HashMap::new();
        for pt in &tpl.properties {
            props.insert(pt.name.clone(), pt.default_value.clone());
        }
        for (name, value) in &snode.properties {
            props.insert(name.clone(), value.clone());
        }

        let debug = props.get("Debug").map(|v| v == "true").unwrap_or(false);
        let block_name = format!("{}_{}", tpl.ref_name, snode.id);

        let block = Block {
            id: snode.id,
            name: block_name,
            debug,
            node_indices: HashMap::new(),
            properties: props,
        };
        graph.blocks.insert(snode.id, block);
        graph.block_templates.insert(snode.id, tpl.ref_name.clone());

        // Add lifecycle script nodes
        fn add_script_node(
            graph: &mut CompGraph,
            block_id: u32,
            name: &str,
            flags: u32,
            script: &Option<String>,
        ) {
            if let Some(ref s) = script {
                if !s.is_empty() {
                    let node = CompNode {
                        block_id,
                        name: name.to_string(),
                        node_type: ScriptNodeType::Script,
                        flags: NF::FORCE | flags,
                        script: s.clone(),
                        compiled_script: String::new(),
                        out_links: Vec::new(),
                        in_links: Vec::new(),
                    };
                    graph.add_node(block_id, node);
                }
            }
        }

        add_script_node(&mut graph, snode.id, "StaticInitScript", NF::STATIC_INIT_SCRIPT, &tpl.static_init_script);
        add_script_node(&mut graph, snode.id, "StaticTeardownScript", NF::STATIC_TEARDOWN_SCRIPT, &tpl.static_teardown_script);
        add_script_node(&mut graph, snode.id, "PreInitScript", NF::PRE_INIT_SCRIPT, &tpl.pre_init_script);
        add_script_node(&mut graph, snode.id, "InitScript", NF::INIT_SCRIPT, &tpl.init_script);
        add_script_node(&mut graph, snode.id, "TeardownScript", NF::TEARDOWN_SCRIPT, &tpl.teardown_script);
        add_script_node(&mut graph, snode.id, "PersistScript", NF::PERSIST_SCRIPT, &tpl.persist_script);
        add_script_node(&mut graph, snode.id, "RestoreScript", NF::RESTORE_SCRIPT, &tpl.restore_script);

        // Named methods
        for method in &tpl.methods {
            let node = CompNode {
                block_id: snode.id,
                name: method.name.clone(),
                node_type: ScriptNodeType::Script,
                flags: NF::FORCE | NF::NAMED_METHOD | NF::DONT_INLINE,
                script: method.script.clone(),
                compiled_script: String::new(),
                out_links: Vec::new(),
                in_links: Vec::new(),
            };
            graph.add_node(snode.id, node);
        }

        // Port nodes
        for pt in &tpl.ports {
            let mut flags: u32 = 0;
            if pt.is_trigger() {
                flags |= NF::TRIGGER;
            }
            if pt.required {
                flags |= NF::REQUIRED;
            }
            let direction = if pt.direction == "out" {
                ScriptNodeType::OutputPort
            } else {
                ScriptNodeType::InputPort
            };
            let port_script = if pt.direction == "in" && pt.is_trigger() {
                pt.script.clone().unwrap_or_default()
            } else {
                String::new()
            };
            let node = CompNode {
                block_id: snode.id,
                name: pt.name.clone(),
                node_type: direction,
                flags,
                script: port_script,
                compiled_script: String::new(),
                out_links: Vec::new(),
                in_links: Vec::new(),
            };
            graph.add_node(snode.id, node);
        }
    }

    // Pass 2: Add connections from the script file
    for conn in &script.connections {
        let src_block = graph.blocks.get(&conn.out_node);
        let tgt_block = graph.blocks.get(&conn.in_node);
        if src_block.is_none() || tgt_block.is_none() {
            tracing::warn!(
                "Connection references unknown node(s): {} -> {}",
                conn.out_node,
                conn.in_node
            );
            continue;
        }
        let src_idx = match src_block.unwrap().port(&conn.out_port) {
            Some(i) => i,
            None => {
                tracing::warn!(
                    "Connection references unknown port '{}' on node {}",
                    conn.out_port,
                    conn.out_node
                );
                continue;
            }
        };
        let tgt_idx = match tgt_block.unwrap().port(&conn.in_port) {
            Some(i) => i,
            None => {
                tracing::warn!(
                    "Connection references unknown port '{}' on node {}",
                    conn.in_port,
                    conn.in_node
                );
                continue;
            }
        };
        graph.add_connection(src_idx, tgt_idx, CF::CONNECTED | CF::PERSISTENT);
    }

    Ok(graph)
}

// ---------------------------------------------------------------------------
// Optimization (mirrors ScriptOptimizer::optimize)
// ---------------------------------------------------------------------------

fn optimize_and_compile(graph: &mut CompGraph) -> Result<()> {
    // Remove non-persistent connections added during previous compilation
    remove_non_persistent_connections(graph);

    // Initial compilation pass: discover all dependencies
    let all_nodes: Vec<usize> = (0..graph.nodes.len()).collect();
    for &ni in &all_nodes {
        if graph.nodes[ni].is_alive() {
            compile_node(graph, ni, true, false);
        }
    }

    // Find compilation order via topological sort
    find_compilation_order(graph)?;

    // Clear dynamic flags
    for node in &mut graph.nodes {
        node.flags &= !NF::DYNAMIC;
    }
    for conn in &mut graph.connections {
        conn.flags &= !CF::DYNAMIC;
    }

    // Build reverse-dependency order
    let mut dependent_order: Vec<usize> = graph.ordered_nodes.clone();
    dependent_order.reverse();

    let mut iters = 0usize;
    loop {
        // Compilation step
        for &ni in &dependent_order {
            if graph.nodes[ni].is_alive() {
                graph.nodes[ni].flags &= !(NF::COMPILED | NF::INLINED);
                compile_node(graph, ni, true, true);
            }
        }

        // Optimization step
        let mut optimized_once = false;
        loop {
            let mut optimized = false;
            for &ni in &dependent_order {
                if graph.nodes[ni].is_alive() {
                    optimized |= optimize_node(graph, ni);
                }
            }
            optimized_once |= optimized;
            iters += 1;
            if !optimized || iters >= MAX_OPTIMIZATION_ITERATIONS {
                break;
            }
        }

        if !optimized_once || iters >= MAX_OPTIMIZATION_ITERATIONS {
            break;
        }
    }

    Ok(())
}

fn remove_non_persistent_connections(graph: &mut CompGraph) {
    // Mark non-persistent connections for removal
    let to_remove: Vec<usize> = graph
        .connections
        .iter()
        .enumerate()
        .filter(|(_, c)| c.flags & CF::PERSISTENT == 0)
        .map(|(i, _)| i)
        .collect();

    // Remove from node link lists
    for &ci in to_remove.iter().rev() {
        let src = graph.connections[ci].source;
        let tgt = graph.connections[ci].target;
        graph.nodes[src].out_links.retain(|&x| x != ci);
        graph.nodes[tgt].in_links.retain(|&x| x != ci);
    }

    // We can't easily remove from the vec without invalidating indices.
    // Instead, mark them as eliminated.
    for &ci in &to_remove {
        graph.connections[ci].flags |= CF::ELIMINATE;
    }
}

fn find_compilation_order(graph: &mut CompGraph) -> Result<()> {
    let node_count = graph.nodes.len();
    let mut processed = HashSet::new();
    let mut ordered = Vec::new();
    let mut last_processed = 0usize;

    loop {
        for ni in 0..node_count {
            if processed.contains(&ni) {
                continue;
            }
            // All inbound sources must be processed
            let all_deps_ready = graph.nodes[ni]
                .in_links
                .iter()
                .all(|&ci| {
                    let c = &graph.connections[ci];
                    c.flags & CF::ELIMINATE != 0 || processed.contains(&c.source)
                });

            if !all_deps_ready {
                continue;
            }

            processed.insert(ni);
            ordered.push(ni);
        }

        if processed.len() == last_processed && !graph.nodes.is_empty() {
            // Circular dependency detected -- still complete what we can
            tracing::warn!(
                "Circular dependency detected in script graph ({}/{} nodes ordered)",
                ordered.len(),
                node_count
            );
            // Add remaining nodes in arbitrary order to avoid a hard failure
            for ni in 0..node_count {
                if !processed.contains(&ni) {
                    ordered.push(ni);
                }
            }
            break;
        }
        last_processed = processed.len();

        if ordered.len() >= node_count {
            break;
        }
    }

    graph.ordered_nodes = ordered;
    Ok(())
}

// ---------------------------------------------------------------------------
// Connection-level optimization (mirrors ScriptConnection::optimize)
// ---------------------------------------------------------------------------

fn optimize_connection(graph: &mut CompGraph, ci: usize) -> bool {
    let c = &graph.connections[ci];
    if !c.is_alive() {
        return false;
    }

    let src = c.source;
    let tgt = c.target;
    let flags = c.flags;

    // A --(T)--> B and B unconnected and has no script: eliminate
    if flags & CF::TRIGGER != 0
        && !graph.has_outbound_links_all(tgt, 0, 0)
        && graph.nodes[tgt].is_compiled()
        && graph.nodes[tgt].compiled_script.is_empty()
    {
        graph.nodes[tgt].flags |= NF::ELIMINATE;
        graph.connections[ci].flags |= CF::ELIMINATE;
        return true;
    }

    // A --(P)--> B and (B unconnected or B has no write/access links): eliminate
    if flags & CF::PROPAGATE != 0
        && !graph.has_inbound_links_any(tgt, CF::READ | CF::ACCESS, 0)
        && !graph.has_outbound_links_all(tgt, 0, 0)
    {
        graph.nodes[tgt].flags |= NF::ELIMINATE;
        graph.connections[ci].flags |= CF::ELIMINATE;
        return true;
    }

    // A --(W)--> B and B has no read/access links and B unconnected: eliminate
    if flags & CF::WRITE != 0
        && !graph.has_outbound_links_all(tgt, 0, 0)
        && !graph.has_inbound_links_any(tgt, CF::READ | CF::ACCESS, 0)
    {
        graph.nodes[tgt].flags |= NF::ELIMINATE;
        graph.connections[ci].flags |= CF::ELIMINATE;
        return true;
    }

    // A --(C)--> B and A has no T/P links: eliminate link
    if flags & CF::CONNECTED != 0
        && !graph.has_inbound_links_any(src, CF::TRIGGER | CF::PROPAGATE, 0)
    {
        graph.connections[ci].flags |= CF::ELIMINATE;
        return true;
    }

    // A --(R)--> B and B has no write/access/inbound connections: eliminate
    if flags & CF::READ != 0
        && !graph.has_inbound_links_any(tgt, CF::WRITE | CF::ACCESS | CF::PROPAGATE, 0)
    {
        graph.nodes[tgt].flags |= NF::ELIMINATE;
        graph.connections[ci].flags |= CF::ELIMINATE;
        return true;
    }

    // Source eliminated -> eliminate link
    if !graph.nodes[src].is_alive() {
        graph.connections[ci].flags |= CF::ELIMINATE;
        return true;
    }

    false
}

fn optimize_node(graph: &mut CompGraph, ni: usize) -> bool {
    let mut optimized = false;

    // Optimize outbound connections
    let out_links: Vec<usize> = graph.nodes[ni].out_links.clone();
    for ci in out_links {
        if graph.connections[ci].is_alive() {
            optimized |= optimize_connection(graph, ci);
        }
    }

    // Try eliminating this node if it isn't referenced
    if graph.nodes[ni].flags & (NF::FORCE | NF::CALLED) == 0 {
        let mut alive_conns = 0;
        for &ci in &graph.nodes[ni].out_links {
            if graph.connections[ci].is_alive() {
                alive_conns += 1;
            }
        }
        for &ci in &graph.nodes[ni].in_links {
            if graph.connections[ci].is_alive() {
                alive_conns += 1;
            }
        }
        if alive_conns == 0 {
            graph.nodes[ni].flags |= NF::ELIMINATE;
            optimized = true;
        }
    }

    optimized
}

// ---------------------------------------------------------------------------
// Node compilation (mirrors ScriptNodeCompiler)
// ---------------------------------------------------------------------------

fn compile_node(graph: &mut CompGraph, ni: usize, allow_inlining: bool, ordered: bool) {
    debug_assert!(!graph.nodes[ni].is_compiled());
    let compiled = compile_body(graph, ni, ni, allow_inlining, ordered);
    let compiled_text = compiled.join("\r\n");
    graph.nodes[ni].compiled_script = compiled_text;
    graph.nodes[ni].flags |= NF::COMPILED;
    if compiled.len() <= MAX_INLINE_LENGTH && graph.nodes[ni].flags & NF::DONT_INLINE == 0 {
        graph.nodes[ni].flags |= NF::INLINED;
    }
}

fn compile_body(
    graph: &mut CompGraph,
    context_ni: usize, // the node being compiled (for dependency tracking)
    port_ni: usize,    // the node whose body we're compiling
    allow_inlining: bool,
    ordered: bool,
) -> Vec<String> {
    let node_type = graph.nodes[port_ni].node_type;
    let flags = graph.nodes[port_ni].flags;

    if node_type == ScriptNodeType::OutputPort {
        if flags & NF::TRIGGER != 0 {
            // Trigger output port: emit trigger calls
            let out_links: Vec<usize> = graph.nodes[port_ni].out_links.clone();
            let mut body = Vec::new();
            for ci in out_links {
                if graph.connections[ci].is_alive()
                    && graph.connections[ci].flags & CF::CONNECTED != 0
                {
                    let target = graph.connections[ci].target;
                    graph.add_dependency(context_ni, target, CF::TRIGGER);
                    let inlined = ordered && graph.nodes[target].flags & NF::INLINED != 0;
                    if !inlined {
                        let fname = compile_function_name(graph, target);
                        body.push(format!("self.{}()", fname));
                    } else {
                        let script = graph.nodes[target].compiled_script.clone();
                        body.extend(reindent_script_str(&script, 0));
                    }
                }
            }
            body
        } else {
            // Propagator output port: assign + propagate to connected inputs
            let out_links: Vec<usize> = graph.nodes[port_ni].out_links.clone();
            let srcprop = format!("self.{}", compile_name_node(graph, port_ni));
            let mut body = Vec::new();
            for ci in out_links {
                if graph.connections[ci].is_alive()
                    && graph.connections[ci].flags & CF::CONNECTED != 0
                {
                    let target = graph.connections[ci].target;
                    graph.add_dependency(context_ni, target, CF::PROPAGATE);
                    let tgt_name = compile_name_node(graph, target);
                    body.push(format!("self.{} = {}", tgt_name, srcprop));
                    let tgt_script = graph.nodes[target].compiled_script.clone();
                    body.extend(reindent_script_str(&tgt_script, 0));
                }
            }
            body
        }
    } else {
        // Script / InputPort: compile the text script with substitutions
        let script_text = graph.nodes[port_ni].script.clone();
        let block_id = graph.nodes[port_ni].block_id;
        compile_script_text(graph, context_ni, block_id, &script_text, 0, allow_inlining, ordered)
    }
}

// ---------------------------------------------------------------------------
// Script text compilation with substitution + preprocessor
// ---------------------------------------------------------------------------

fn compile_script_text(
    graph: &mut CompGraph,
    context_ni: usize,
    block_id: u32,
    script: &str,
    indentation: usize,
    allow_inlining: bool,
    ordered: bool,
) -> Vec<String> {
    let lines = reindent_script_str(script, indentation);
    if lines.is_empty() {
        return Vec::new();
    }

    let mut compiled = Vec::new();
    let mut precomp_levels: Vec<bool> = Vec::new();
    let mut precomp_false_levels: i32 = 0;

    for line in &lines {
        let trimmed = line.trim();

        // Check for preprocessor directives
        if let Some(directive) = parse_preproc_directive(trimmed) {
            match directive.insn.as_str() {
                "if" | "ifn" => {
                    compile_preproc_conditional(
                        graph,
                        block_id,
                        &directive.insn,
                        &directive.args,
                        &mut precomp_levels,
                        &mut precomp_false_levels,
                    );
                }
                "else" | "elseif" | "elseifn" => {
                    if !precomp_levels.is_empty() {
                        if *precomp_levels.last().unwrap() {
                            precomp_false_levels += 1;
                        } else {
                            precomp_false_levels -= 1;
                        }
                        let last = precomp_levels.last_mut().unwrap();
                        *last = !*last;

                        if directive.insn == "elseif" || directive.insn == "elseifn" {
                            if !*precomp_levels.last().unwrap() {
                                precomp_false_levels -= 1;
                            }
                            precomp_levels.pop();
                            compile_preproc_conditional(
                                graph,
                                block_id,
                                &directive.insn,
                                &directive.args,
                                &mut precomp_levels,
                                &mut precomp_false_levels,
                            );
                        }
                    } else {
                        tracing::warn!("#ELSE without matching #IF in block {}", block_id);
                    }
                }
                "endif" => {
                    if !precomp_levels.is_empty() {
                        if !*precomp_levels.last().unwrap() {
                            precomp_false_levels -= 1;
                        }
                        precomp_levels.pop();
                    } else {
                        tracing::warn!("#ENDIF without matching #IF in block {}", block_id);
                    }
                }
                other => {
                    tracing::warn!("Unknown preprocessor directive: #{}", other);
                }
            }
            continue;
        }

        // Expression substitution
        let mut current_line = line.clone();
        let mut add_line = true;

        // Repeatedly substitute KEYWORD{Name} expressions
        loop {
            let Some((full_match, start, expr_type, variant, name)) =
                find_expression(&current_line)
            else {
                break;
            };

            let replacement = compile_expression(
                graph,
                context_ni,
                block_id,
                &expr_type,
                &variant,
                &name,
                allow_inlining,
                ordered,
            );

            if replacement.is_empty() {
                current_line.clear();
                add_line = false;
                break;
            } else if replacement.len() == 1 {
                current_line =
                    format!("{}{}{}", &current_line[..start], replacement[0], &current_line[start + full_match.len()..]);
            } else {
                // Multi-line expansion: only allowed at start of line (after indentation)
                let prefix = &current_line[..start];
                let is_only_tabs = prefix.chars().all(|c| c == '\t');
                if !is_only_tabs {
                    tracing::warn!(
                        "Expression expands to multiple lines in non-indentation context"
                    );
                    break;
                }

                if precomp_false_levels == 0 && !current_line.is_empty() {
                    let indented = reindent_script_lines(&replacement, start);
                    compiled.extend(indented);
                }
                add_line = false;
                break;
            }
        }

        if precomp_false_levels == 0 && !current_line.is_empty() && add_line {
            compiled.push(current_line);
        }
    }

    if !precomp_levels.is_empty() {
        tracing::warn!(
            "{} preprocessor blocks were not closed in block {}",
            precomp_levels.len(),
            block_id
        );
    }

    compiled
}

struct PreprocDirective {
    insn: String,
    args: String,
}

fn parse_preproc_directive(line: &str) -> Option<PreprocDirective> {
    let trimmed = line.trim();
    if !trimmed.starts_with('#') {
        return None;
    }
    let rest = trimmed[1..].trim();
    // Split on first whitespace
    let (insn, args) = match rest.find(|c: char| c.is_whitespace()) {
        Some(pos) => (&rest[..pos], rest[pos..].trim()),
        None => (rest, ""),
    };
    Some(PreprocDirective {
        insn: insn.to_lowercase(),
        args: args.to_string(),
    })
}

fn compile_preproc_conditional(
    graph: &CompGraph,
    block_id: u32,
    insn: &str,
    args: &str,
    precomp_levels: &mut Vec<bool>,
    precomp_false_levels: &mut i32,
) {
    let negate = insn == "ifn" || insn == "elseifn";

    if args == "DEBUG" {
        let block = graph.blocks.get(&block_id);
        let debug = block.map(|b| b.debug).unwrap_or(false);
        let condition_true = debug ^ negate;
        if !condition_true {
            precomp_levels.push(false);
            *precomp_false_levels += 1;
        } else {
            precomp_levels.push(true);
        }
        return;
    }

    // CONNECTED(Port Name)
    if let Some(port_name) = parse_connected_expr(args) {
        let block = graph.blocks.get(&block_id);
        let mut connected = false;
        if let Some(block) = block {
            if let Some(ni) = block.port(&port_name) {
                let node = &graph.nodes[ni];
                if node.node_type == ScriptNodeType::InputPort {
                    connected = graph.has_inbound_links_all(ni, CF::CONNECTED, 0);
                } else if node.node_type == ScriptNodeType::OutputPort {
                    connected = graph.has_outbound_links_all(ni, CF::CONNECTED, 0);
                }
            }
        }
        let condition_true = connected ^ negate;
        if !condition_true {
            precomp_levels.push(false);
            *precomp_false_levels += 1;
        } else {
            precomp_levels.push(true);
        }
        return;
    }

    // PROPERTY(Name) == "value"
    if let Some((prop_name, expected)) = parse_property_expr(args) {
        let block = graph.blocks.get(&block_id);
        let actual = block.and_then(|b| b.property(&prop_name)).cloned().unwrap_or_default();
        let matches = actual == expected;
        let condition_true = matches ^ negate;
        if !condition_true {
            precomp_levels.push(false);
            *precomp_false_levels += 1;
        } else {
            precomp_levels.push(true);
        }
        return;
    }

    tracing::warn!("#IF: unrecognized expression '{}' in block {}", args, block_id);
    precomp_levels.push(true); // default: include
}

fn parse_connected_expr(args: &str) -> Option<String> {
    let trimmed = args.trim();
    if !trimmed.to_uppercase().starts_with("CONNECTED(") {
        return None;
    }
    let start = "CONNECTED(".len();
    let end = trimmed.find(')')?;
    Some(trimmed[start..end].trim().to_string())
}

fn parse_property_expr(args: &str) -> Option<(String, String)> {
    let trimmed = args.trim();
    if !trimmed.to_uppercase().starts_with("PROPERTY(") {
        return None;
    }
    let paren_end = trimmed.find(')')?;
    let prop_name = trimmed["PROPERTY(".len()..paren_end].trim().to_string();
    let rest = trimmed[paren_end + 1..].trim();
    // Expect == "value"
    let rest = rest.strip_prefix("==")?;
    let rest = rest.trim();
    let rest = rest.strip_prefix('"')?;
    let end_quote = rest.find('"')?;
    let value = rest[..end_quote].to_string();
    Some((prop_name, value))
}

/// Find the next KEYWORD{Name} expression in a line.
/// Returns (full_match_str, start_pos, type, variant, name).
fn find_expression(line: &str) -> Option<(String, usize, String, String, String)> {
    // Pattern: [a-zA-Z_.]+{[^{}]*}
    let bytes = line.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    while i < len {
        // Look for '{'
        if bytes[i] == b'{' {
            // Scan backward for the keyword part
            let brace_pos = i;
            let mut kw_start = i;
            while kw_start > 0 {
                let c = bytes[kw_start - 1];
                if c.is_ascii_alphabetic() || c == b'_' || c == b'.' {
                    kw_start -= 1;
                } else {
                    break;
                }
            }
            if kw_start == brace_pos {
                i += 1;
                continue;
            }

            // Scan forward for closing '}'
            let mut j = brace_pos + 1;
            while j < len && bytes[j] != b'}' && bytes[j] != b'{' {
                j += 1;
            }
            if j < len && bytes[j] == b'}' {
                let full = &line[kw_start..=j];
                let keyword = &line[kw_start..brace_pos];
                let name = &line[brace_pos + 1..j];

                // Split keyword on '.' for variant
                let (expr_type, variant) = match keyword.find('.') {
                    Some(dot) => (keyword[..dot].to_string(), keyword[dot + 1..].to_string()),
                    None => (keyword.to_string(), String::new()),
                };

                return Some((
                    full.to_string(),
                    kw_start,
                    expr_type,
                    variant,
                    name.to_string(),
                ));
            }
            i = j + 1;
        } else {
            i += 1;
        }
    }
    None
}

fn compile_expression(
    graph: &mut CompGraph,
    context_ni: usize,
    block_id: u32,
    expr_type: &str,
    variant: &str,
    name: &str,
    _allow_inlining: bool,
    ordered: bool,
) -> Vec<String> {
    match expr_type {
        "TRIGGER" => compile_trigger_expr(graph, context_ni, block_id, variant, name, ordered),
        "VAR" => compile_var_expr(graph, context_ni, block_id, variant, name),
        "PROPERTY" => compile_property_expr(graph, block_id, name),
        "VAR_OR_PROP" => compile_var_or_prop_expr(graph, context_ni, block_id, name),
        "PROPAGATE" => compile_propagate_expr(graph, context_ni, block_id, variant, name, ordered),
        "LOCAL" => {
            let block = graph.blocks.get(&block_id);
            let id = block.map(|b| b.id).unwrap_or(0);
            vec![format!("self.n{}_lvar_{}", id, sanitize_name(name))]
        }
        "NODEID" => {
            let block = graph.blocks.get(&block_id);
            let id = block.map(|b| b.id).unwrap_or(0);
            vec![id.to_string()]
        }
        "NODENAME" => {
            let block = graph.blocks.get(&block_id);
            vec![block.map(|b| b.name.clone()).unwrap_or_default()]
        }
        _ => {
            tracing::warn!("Unknown expression type: '{}'", expr_type);
            vec!["<<TYPE-ERROR>>".to_string()]
        }
    }
}

fn compile_trigger_expr(
    graph: &mut CompGraph,
    context_ni: usize,
    block_id: u32,
    variant: &str,
    name: &str,
    _ordered: bool,
) -> Vec<String> {
    let block = match graph.blocks.get(&block_id) {
        Some(b) => b,
        None => return vec!["<<TRIGGER-ERROR>>".to_string()],
    };
    let port_ni = match block.port(name) {
        Some(ni) => ni,
        None => {
            tracing::warn!("Trigger port '{}' not found in block {}", name, block_id);
            return vec!["<<TRIGGER-ERROR>>".to_string()];
        }
    };

    if graph.nodes[port_ni].node_type != ScriptNodeType::OutputPort
        || graph.nodes[port_ni].flags & NF::TRIGGER == 0
    {
        tracing::warn!("'{}' is not an output trigger port", name);
        return vec!["<<TRIGGER-ERROR>>".to_string()];
    }

    graph.add_dependency(context_ni, port_ni, CF::TRIGGER);

    let pass = variant.contains('P');
    let force_inline = variant.contains('I') && graph.nodes[port_ni].flags & NF::DONT_INLINE == 0;
    let force_call = variant.contains('F');

    if force_call && graph.nodes[port_ni].flags & NF::CALLED == 0 {
        graph.nodes[port_ni].flags |= NF::CALLED;
    }

    let compiled = graph.nodes[port_ni].compiled_script.clone();
    if !compiled.is_empty() {
        let mut inlined = force_inline;
        let mut called = force_call;
        if !inlined && !called {
            if graph.nodes[port_ni].flags & NF::INLINED != 0 {
                inlined = true;
            } else {
                called = true;
            }
        }
        let _ = called; // used for control flow above

        if inlined {
            return compiled.split("\r\n").map(|s| s.to_string()).collect();
        } else {
            let fname = compile_function_name(graph, port_ni);
            return vec![format!("self.{}()", fname)];
        }
    } else if pass {
        return vec!["pass".to_string()];
    }

    Vec::new()
}

fn compile_var_expr(
    graph: &mut CompGraph,
    context_ni: usize,
    block_id: u32,
    variant: &str,
    name: &str,
) -> Vec<String> {
    let mut dep_flags = 0u32;
    if variant.contains('R') {
        dep_flags |= CF::READ;
    }
    if variant.contains('W') {
        dep_flags |= CF::WRITE;
    }
    if variant.contains('A') {
        dep_flags |= CF::ACCESS;
    }
    if dep_flags == 0 {
        dep_flags = CF::ACCESS;
    }

    let block = match graph.blocks.get(&block_id) {
        Some(b) => b,
        None => return vec!["<<VAR-ERROR>>".to_string()],
    };
    let port_ni = match block.port(name) {
        Some(ni) => ni,
        None => {
            tracing::warn!("Variable port '{}' not found in block {}", name, block_id);
            return vec!["<<VAR-ERROR>>".to_string()];
        }
    };

    if graph.nodes[port_ni].flags & NF::TRIGGER != 0 {
        tracing::warn!("'{}' is a trigger port, not a variable port", name);
        return vec!["<<VAR-ERROR>>".to_string()];
    }

    graph.add_dependency(context_ni, port_ni, dep_flags);

    // Don't emit code if writing to an eliminated variable
    if dep_flags == CF::WRITE && !graph.nodes[port_ni].is_alive() {
        return Vec::new();
    }
    if dep_flags == CF::READ && !graph.nodes[port_ni].is_alive() {
        return vec!["None".to_string()];
    }

    vec![format!("self.{}", compile_name_node(graph, port_ni))]
}

fn compile_property_expr(graph: &CompGraph, block_id: u32, name: &str) -> Vec<String> {
    if is_internal_property(name) {
        tracing::warn!("Attempted to read internal property '{}'", name);
        return vec!["<<PROPERTY-ERROR>>".to_string()];
    }
    let block = match graph.blocks.get(&block_id) {
        Some(b) => b,
        None => return vec!["<<PROPERTY-ERROR>>".to_string()],
    };
    match block.property(name) {
        Some(val) => vec![to_script_string(val)],
        None => {
            tracing::warn!("Property '{}' not found in block {}", name, block_id);
            vec!["<<PROPERTY-ERROR>>".to_string()]
        }
    }
}

fn compile_var_or_prop_expr(
    graph: &mut CompGraph,
    context_ni: usize,
    block_id: u32,
    name: &str,
) -> Vec<String> {
    let block = match graph.blocks.get(&block_id) {
        Some(b) => b,
        None => return vec!["<<VAR-ERROR>>".to_string()],
    };
    let port_ni = match block.port(name) {
        Some(ni) => ni,
        None => {
            tracing::warn!("Variable port '{}' not found in block {}", name, block_id);
            return vec!["<<VAR-ERROR>>".to_string()];
        }
    };
    if is_internal_property(name) {
        return vec!["<<PROPERTY-ERROR>>".to_string()];
    }
    let prop_val = match block.property(name) {
        Some(v) => v.clone(),
        None => {
            return vec!["<<PROPERTY-ERROR>>".to_string()];
        }
    };

    graph.add_dependency(context_ni, port_ni, CF::READ);

    if !graph.nodes[port_ni].is_alive() {
        vec![to_script_string(&prop_val)]
    } else {
        vec![format!(
            "(self.{} or {})",
            compile_name_node(graph, port_ni),
            to_script_string(&prop_val)
        )]
    }
}

fn compile_propagate_expr(
    graph: &mut CompGraph,
    context_ni: usize,
    block_id: u32,
    variant: &str,
    name: &str,
    _ordered: bool,
) -> Vec<String> {
    let block = match graph.blocks.get(&block_id) {
        Some(b) => b,
        None => return vec!["<<PROPAGATE-ERROR>>".to_string()],
    };
    let port_ni = match block.port(name) {
        Some(ni) => ni,
        None => {
            tracing::warn!("Output port '{}' not found in block {}", name, block_id);
            return vec!["<<PROPAGATE-ERROR>>".to_string()];
        }
    };

    if graph.nodes[port_ni].node_type != ScriptNodeType::OutputPort
        || graph.nodes[port_ni].flags & NF::TRIGGER != 0
    {
        tracing::warn!("'{}' is not an output variable port", name);
        return vec!["<<PROPAGATE-ERROR>>".to_string()];
    }

    graph.add_dependency(context_ni, port_ni, CF::PROPAGATE);

    if !graph.nodes[port_ni].is_alive() {
        return Vec::new();
    }

    let pass = variant.contains('P');
    let force_inline = variant.contains('I') && graph.nodes[port_ni].flags & NF::DONT_INLINE == 0;
    let force_call = variant.contains('F');

    if force_call && graph.nodes[port_ni].flags & NF::CALLED == 0 {
        graph.nodes[port_ni].flags |= NF::CALLED;
    }

    let compiled = graph.nodes[port_ni].compiled_script.clone();
    if !compiled.is_empty() {
        let mut inlined = force_inline;
        let mut called = force_call;
        if !inlined && !called {
            if graph.nodes[port_ni].flags & NF::INLINED != 0 {
                inlined = true;
            } else {
                called = true;
            }
        }
        let _ = called;

        if inlined {
            return compiled.split("\r\n").map(|s| s.to_string()).collect();
        } else {
            let fname = compile_function_name(graph, port_ni);
            return vec![format!("self.{}()", fname)];
        }
    } else if pass {
        return vec!["pass".to_string()];
    }

    Vec::new()
}

// ---------------------------------------------------------------------------
// Name generation (matches C++ ScriptNodeCompiler::compileName / compileFunctionName)
// ---------------------------------------------------------------------------

fn compile_function_name(graph: &CompGraph, ni: usize) -> String {
    let node = &graph.nodes[ni];
    match node.node_type {
        ScriptNodeType::OutputPort => {
            format!("n{}_propagator_{}", node.block_id, sanitize_name(&node.name))
        }
        ScriptNodeType::InputPort => {
            format!("n{}_trigger_{}", node.block_id, sanitize_name(&node.name))
        }
        ScriptNodeType::Script => sanitize_name(&node.name),
    }
}

fn compile_name_node(graph: &CompGraph, ni: usize) -> String {
    let node = &graph.nodes[ni];
    let kind = match node.node_type {
        ScriptNodeType::OutputPort | ScriptNodeType::InputPort => "_var_",
        ScriptNodeType::Script => "_script_",
    };
    format!("n{}{}{}", node.block_id, kind, sanitize_name(&node.name))
}

/// Convert a property/port name to a Python-safe identifier.
/// Matches the C++ ScriptNodeCompiler::compileName(QString) exactly.
fn sanitize_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for c in name.chars() {
        match c {
            ' ' | '\t' | '_' => out.push('_'),
            '<' => out.push_str("less_"),
            '>' => out.push_str("greater_"),
            '=' => out.push_str("equal_"),
            '!' => out.push_str("not_"),
            '&' => out.push_str("and_"),
            '|' => out.push_str("or_"),
            c if c.is_ascii_alphanumeric() => out.push(c),
            _ => out.push('_'),
        }
    }
    out
}

fn is_internal_property(name: &str) -> bool {
    name == "Comment" || name == "Enabled"
}

/// Convert a property value to its Python representation.
/// Matches the C++ ScriptNodeCompiler::toScriptString.
fn to_script_string(value: &str) -> String {
    match value {
        "true" | "True" => "True".to_string(),
        "false" | "False" => "False".to_string(),
        _ => {
            // Try to parse as a number
            if value.parse::<i64>().is_ok() || value.parse::<f64>().is_ok() {
                value.to_string()
            } else {
                // String value -- quote it
                let escaped = value.replace('\\', "\\\\").replace('\'', "\\'");
                format!("'{}'", escaped)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Re-indentation (matches C++ ScriptNodeCompiler::reindentScript)
// ---------------------------------------------------------------------------

fn reindent_script_str(script: &str, indentation: usize) -> Vec<String> {
    if script.is_empty() {
        return Vec::new();
    }
    let lines: Vec<&str> = script.split(|c| c == '\r' || c == '\n').filter(|l| !l.is_empty()).collect();
    if lines.is_empty() {
        return Vec::new();
    }
    reindent_lines(&lines, indentation)
}

fn reindent_script_lines(lines: &[String], indentation: usize) -> Vec<String> {
    let refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
    reindent_lines(&refs, indentation)
}

fn reindent_lines(lines: &[&str], indentation: usize) -> Vec<String> {
    if lines.is_empty() {
        return Vec::new();
    }

    // Count leading tabs on first line
    let first = lines[0];
    let tabs = first.chars().take_while(|&c| c == '\t').count();

    let prefix = "\t".repeat(indentation);
    lines
        .iter()
        .map(|line| {
            if line.len() >= tabs {
                format!("{}{}", prefix, &line[tabs.min(line.len())..])
            } else {
                format!("{}{}", prefix, line)
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Python emission (mirrors ScriptOptimizer::compile output section)
// ---------------------------------------------------------------------------

fn emit_python(
    graph: &CompGraph,
    script: &ScriptFile,
    defs: &ScriptDefinitions,
    template_map: &HashMap<&str, &NodeTemplate>,
) -> Result<String> {
    let mut out = String::new();

    let parent_class = if script.script_type == "Effect" {
        "EffectScript"
    } else {
        "Script"
    };

    // Header
    out.push_str("# Automatically generated by Atrea Script Editor\r\n");
    out.push_str(&format!("# Script Version: {}\r\n", script.version));
    out.push_str(&format!(
        "# Dataset Version: {}\r\n",
        defs.dataset_version
    ));
    out.push_str("\r\n");
    out.push_str(&format!(
        "from cell.Script import {}\r\n",
        parent_class
    ));

    // Collect imports from all used block templates
    let mut seen_imports = HashSet::new();
    for snode in &script.nodes {
        if let Some(tpl) = template_map.get(snode.ref_name.as_str()) {
            for import in &tpl.imports {
                if seen_imports.insert(import.clone()) {
                    out.push_str(&format!("{}\r\n", import));
                }
            }
        }
    }

    // Class name from module
    let class_name = script
        .module
        .rsplit('.')
        .next()
        .unwrap_or("UnnamedScriptClass");
    let class_name = if class_name.is_empty() {
        "UnnamedScriptClass"
    } else {
        class_name
    };

    out.push_str("\r\n");
    out.push_str(&format!(
        "class {}({}):\r\n",
        class_name, parent_class
    ));

    // Collect lifecycle scripts
    let mut static_init = String::new();
    let mut static_teardown = String::new();
    let mut pre_init = String::new();
    let mut init_script = String::new();
    let mut teardown = String::new();
    let mut persist = String::new();
    let mut restore = String::new();

    for ni in 0..graph.nodes.len() {
        let node = &graph.nodes[ni];
        if !node.is_alive() {
            continue;
        }

        let is_init = node.flags & (NF::INIT_SCRIPT | NF::PRE_INIT_SCRIPT) != 0;
        let is_static_init = node.flags & NF::STATIC_INIT_SCRIPT != 0;
        let is_static_teardown = node.flags & NF::STATIC_TEARDOWN_SCRIPT != 0;
        let is_teardown = node.flags & NF::TEARDOWN_SCRIPT != 0;
        let is_persist = node.flags & NF::PERSIST_SCRIPT != 0;
        let is_restore = node.flags & NF::RESTORE_SCRIPT != 0;
        let is_named_method = node.flags & NF::NAMED_METHOD != 0;
        let is_port =
            node.node_type == ScriptNodeType::InputPort || node.node_type == ScriptNodeType::OutputPort;
        let is_trigger = node.flags & NF::TRIGGER != 0;

        let reindented = || -> String {
            if node.compiled_script.is_empty() {
                String::new()
            } else {
                let lines = reindent_script_str(&node.compiled_script, 2);
                lines.join("\r\n") + "\r\n"
            }
        };

        if is_init {
            let script = reindented();
            if !script.is_empty() {
                if node.flags & NF::PRE_INIT_SCRIPT != 0 {
                    pre_init.push_str(&script);
                } else {
                    init_script.push_str(&script);
                }
            }
        } else if is_static_init {
            let script = reindented();
            if !script.is_empty() {
                static_init.push_str(&script);
            }
        } else if is_static_teardown {
            let script = reindented();
            if !script.is_empty() {
                static_teardown.push_str(&script);
            }
        } else if is_teardown {
            let script = reindented();
            if !script.is_empty() {
                teardown.push_str(&script);
            }
        } else if is_persist {
            let script = reindented();
            if !script.is_empty() {
                persist.push_str(&script);
            }
        } else if is_restore {
            let script = reindented();
            if !script.is_empty() {
                restore.push_str(&script);
            }
        } else if is_named_method {
            let script = reindented();
            if !script.is_empty() {
                out.push_str(&format!("\tdef {}(self):\r\n", node.name));
                out.push_str(&script);
                out.push_str("\r\n");
            }
        } else if is_port && !is_trigger {
            // Variable port: emit class variable + optional propagator function
            if node.flags & NF::REQUIRED != 0 {
                let connected = if node.node_type == ScriptNodeType::InputPort {
                    graph.has_inbound_links_all(ni, CF::CONNECTED, 0)
                } else {
                    graph.has_outbound_links_all(ni, CF::CONNECTED, 0)
                };
                if !connected {
                    tracing::warn!(
                        "Required port '{}' of block '{}' is not connected",
                        node.name,
                        graph
                            .blocks
                            .get(&node.block_id)
                            .map(|b| b.name.as_str())
                            .unwrap_or("?")
                    );
                }
            }
            out.push_str(&format!(
                "\t{} = None\r\n",
                compile_name_node(graph, ni)
            ));
            if node.node_type == ScriptNodeType::OutputPort && node.should_emit_function() {
                let script = reindented();
                if !script.is_empty() {
                    out.push_str(&format!(
                        "\tdef {}(self):\r\n",
                        compile_function_name(graph, ni)
                    ));
                    out.push_str(&script);
                    out.push_str("\r\n");
                }
            }
        } else if is_port && is_trigger {
            // Trigger port
            if node.should_emit_function() {
                let script = reindented();
                if !script.is_empty() {
                    out.push_str(&format!(
                        "\tdef {}(self):\r\n",
                        compile_function_name(graph, ni)
                    ));
                    out.push_str(&script);
                    out.push_str("\r\n");
                }
            }
        }
    }

    // __init__ method
    if script.script_type == "Effect" {
        out.push_str("\tdef __init__(self, owner, effect, nvps):\r\n");
        out.push_str(&format!(
            "\t\t{}.__init__(self, owner, effect, nvps)\r\n",
            parent_class
        ));
    } else {
        out.push_str("\tdef __init__(self, owner, storedVars):\r\n");
        out.push_str(&format!(
            "\t\t{}.__init__(self, owner, storedVars)\r\n",
            parent_class
        ));
    }
    out.push_str(&pre_init);
    out.push_str(&init_script);
    out.push_str("\r\n");

    // Static init
    if !static_init.is_empty() {
        out.push_str("\t@classmethod\r\n");
        out.push_str("\tdef staticInit(cls):\r\n");
        out.push_str(&static_init);
        out.push_str("\r\n");
    }

    // Static teardown
    if !static_teardown.is_empty() {
        out.push_str("\t@classmethod\r\n");
        out.push_str("\tdef staticTeardown(cls):\r\n");
        out.push_str(&static_teardown);
        out.push_str("\r\n");
    }

    // Teardown
    if !teardown.is_empty() {
        out.push_str("\tdef teardown(self):\r\n");
        out.push_str(&teardown);
        out.push_str("\r\n");
    }

    // Persist
    if !persist.is_empty() {
        out.push_str("\tdef persist(self):\r\n");
        out.push_str(&persist);
        out.push_str("\r\n");
    }

    // Restore
    if !restore.is_empty() {
        out.push_str("\tdef restore(self):\r\n");
        out.push_str(&restore);
        out.push_str("\r\n");
    }

    // Static init call at module level
    if !static_init.is_empty() {
        out.push_str(&format!("\r\n{}.staticInit()\r\n", class_name));
    }

    Ok(out)
}
