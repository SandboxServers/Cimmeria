//! A reader for the two expression strings a SigNoz builder query holds:
//! the filter (`service.name = 'x' AND event IN ('a', 'b')`) and an
//! aggregation (`count()`, `p95(duration_ms)`).
//!
//! It exists for the operator-fixture guard in the parent module, which
//! has to know every key a fixture names. It reads the subset of the
//! SigNoz query language the fixtures in `docs/operations/signoz/` use and
//! refuses everything else: a bare search word, a function call in a
//! filter, `BETWEEN`, a bracketed list, an unterminated string. The guard
//! turns a refusal into a test failure, so a query it cannot read is never
//! passed as "nothing to check".

/// A filter, as written: `NOT` binds tighter than `AND`, `AND` tighter
/// than `OR`.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Expr {
    And(Vec<Expr>),
    Or(Vec<Expr>),
    Not(Box<Expr>),
    Cmp(Clause),
}

/// One comparison: a key, an operator and its literals (none for
/// `EXISTS`, several for `IN`).
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Clause {
    pub(super) key: String,
    pub(super) op: Op,
    pub(super) values: Vec<Literal>,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) enum Op {
    Eq,
    Ne,
    In,
    NotIn,
    Exists,
    NotExists,
    /// An ordering or pattern operator (`<`, `LIKE`, `NOT CONTAINS`, …),
    /// kept as written. Its literal cannot be checked against a closed set.
    Other(String),
}

#[derive(Debug, Clone, PartialEq)]
pub(super) enum Literal {
    /// A quoted string, without its quotes.
    Text(String),
    /// A number or `true` / `false`, as written.
    Bare(String),
}

/// An aggregation: the function and the keys it reads.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Aggregation {
    pub(super) function: String,
    pub(super) keys: Vec<String>,
}

impl Expr {
    /// Every comparison in the filter, wherever it sits.
    pub(super) fn clauses(&self) -> Vec<&Clause> {
        match self {
            Expr::Cmp(clause) => vec![clause],
            Expr::Not(inner) => inner.clauses(),
            Expr::And(parts) | Expr::Or(parts) => parts.iter().flat_map(Expr::clauses).collect(),
        }
    }

    /// The comparisons every matching row must satisfy: the filter itself
    /// if it is one comparison, or the comparisons joined by `AND` at the
    /// top. Nothing under an `OR` or a `NOT` is required.
    pub(super) fn required(&self) -> Vec<&Clause> {
        match self {
            Expr::Cmp(clause) => vec![clause],
            Expr::And(parts) => parts.iter().flat_map(Expr::required).collect(),
            Expr::Or(_) | Expr::Not(_) => Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    /// A key or a keyword.
    Word(String),
    Text(String),
    Number(String),
    /// `=`, `!=`, `<>`, `<`, `<=`, `>`, `>=`.
    Symbol(String),
    Open,
    Close,
    Comma,
}

const KEYWORDS: &[&str] = &[
    "AND", "OR", "NOT", "IN", "LIKE", "ILIKE", "CONTAINS", "REGEXP", "EXISTS", "BETWEEN",
];

fn is_keyword(word: &str) -> bool {
    KEYWORDS.contains(&word.to_ascii_uppercase().as_str())
}

fn lex(text: &str) -> Result<Vec<Token>, String> {
    let chars: Vec<char> = text.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
        } else if c == '(' || c == ')' || c == ',' {
            tokens.push(match c {
                '(' => Token::Open,
                ')' => Token::Close,
                _ => Token::Comma,
            });
            i += 1;
        } else if c == '\'' || c == '"' {
            let mut value = String::new();
            i += 1;
            loop {
                match chars.get(i) {
                    None => return Err(format!("unterminated string in `{text}`")),
                    Some('\\') => {
                        let escaped = chars.get(i + 1).ok_or("dangling backslash")?;
                        value.push(*escaped);
                        i += 2;
                    }
                    Some(end) if *end == c => break,
                    Some(other) => {
                        value.push(*other);
                        i += 1;
                    }
                }
            }
            i += 1;
            tokens.push(Token::Text(value));
        } else if c.is_ascii_alphabetic() || c == '_' {
            let start = i;
            while chars
                .get(i)
                .is_some_and(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '.')
            {
                i += 1;
            }
            tokens.push(Token::Word(chars[start..i].iter().collect()));
        } else if c.is_ascii_digit() || c == '-' {
            let start = i;
            i += 1;
            while chars
                .get(i)
                .is_some_and(|c| c.is_ascii_digit() || *c == '.')
            {
                i += 1;
            }
            let number: String = chars[start..i].iter().collect();
            if number == "-" {
                return Err(format!("stray `-` in `{text}`"));
            }
            tokens.push(Token::Number(number));
        } else if matches!(c, '=' | '!' | '<' | '>') {
            let pair: String = chars[i..chars.len().min(i + 2)].iter().collect();
            let symbol = match pair.as_str() {
                "!=" | "<>" | "<=" | ">=" => pair,
                _ if c == '!' => return Err(format!("stray `!` in `{text}`")),
                _ => c.to_string(),
            };
            i += symbol.len();
            tokens.push(Token::Symbol(symbol));
        } else {
            return Err(format!("unexpected `{c}` in `{text}`"));
        }
    }
    Ok(tokens)
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn next(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.pos).cloned();
        self.pos += 1;
        token
    }

    /// Consume `keyword` if it is next.
    fn eat(&mut self, keyword: &str) -> bool {
        let hit = matches!(self.peek(), Some(Token::Word(w)) if w.eq_ignore_ascii_case(keyword));
        if hit {
            self.pos += 1;
        }
        hit
    }

    fn expect(&mut self, token: Token) -> Result<(), String> {
        match self.next() {
            Some(found) if found == token => Ok(()),
            found => Err(format!("expected {token:?}, found {found:?}")),
        }
    }

    fn or(&mut self) -> Result<Expr, String> {
        let mut parts = vec![self.and()?];
        while self.eat("OR") {
            parts.push(self.and()?);
        }
        Ok(match parts.len() {
            1 => parts.remove(0),
            _ => Expr::Or(parts),
        })
    }

    fn and(&mut self) -> Result<Expr, String> {
        let mut parts = vec![self.unary()?];
        while self.eat("AND") {
            parts.push(self.unary()?);
        }
        Ok(match parts.len() {
            1 => parts.remove(0),
            _ => Expr::And(parts),
        })
    }

    fn unary(&mut self) -> Result<Expr, String> {
        if self.eat("NOT") {
            return Ok(Expr::Not(Box::new(self.unary()?)));
        }
        if self.peek() == Some(&Token::Open) {
            self.pos += 1;
            let inner = self.or()?;
            self.expect(Token::Close)?;
            return Ok(inner);
        }
        self.comparison().map(Expr::Cmp)
    }

    fn key(&mut self) -> Result<String, String> {
        match self.next() {
            Some(Token::Word(word)) if !is_keyword(&word) => Ok(word),
            found => Err(format!("expected a key, found {found:?}")),
        }
    }

    fn literal(&mut self) -> Result<Literal, String> {
        match self.next() {
            Some(Token::Text(text)) => Ok(Literal::Text(text)),
            Some(Token::Number(number)) => Ok(Literal::Bare(number)),
            Some(Token::Word(word)) if matches!(word.as_str(), "true" | "false") => {
                Ok(Literal::Bare(word))
            }
            found => Err(format!("expected a literal, found {found:?}")),
        }
    }

    fn comparison(&mut self) -> Result<Clause, String> {
        let key = self.key()?;
        if let Some(Token::Symbol(symbol)) = self.peek().cloned() {
            self.pos += 1;
            let op = match symbol.as_str() {
                "=" => Op::Eq,
                "!=" | "<>" => Op::Ne,
                _ => Op::Other(symbol),
            };
            let values = vec![self.literal()?];
            return Ok(Clause { key, op, values });
        }
        let negated = self.eat("NOT");
        if self.eat("EXISTS") {
            let op = if negated { Op::NotExists } else { Op::Exists };
            return Ok(Clause {
                key,
                op,
                values: Vec::new(),
            });
        }
        if self.eat("IN") {
            self.expect(Token::Open)?;
            let mut values = vec![self.literal()?];
            while self.peek() == Some(&Token::Comma) {
                self.pos += 1;
                values.push(self.literal()?);
            }
            self.expect(Token::Close)?;
            let op = if negated { Op::NotIn } else { Op::In };
            return Ok(Clause { key, op, values });
        }
        for pattern in ["LIKE", "ILIKE", "CONTAINS", "REGEXP"] {
            if self.eat(pattern) {
                let not = if negated { "NOT " } else { "" };
                let values = vec![self.literal()?];
                return Ok(Clause {
                    key,
                    op: Op::Other(format!("{not}{pattern}")),
                    values,
                });
            }
        }
        Err(format!(
            "no operator this guard reads after `{key}`: {:?}",
            self.peek()
        ))
    }
}

/// Read a filter expression. An empty one is an error: every fixture query
/// names at least its service.
pub(super) fn parse_filter(text: &str) -> Result<Expr, String> {
    let tokens = lex(text)?;
    if tokens.is_empty() {
        return Err("empty filter".into());
    }
    let mut parser = Parser { tokens, pos: 0 };
    let expr = parser.or().map_err(|e| format!("{e} in `{text}`"))?;
    match parser.peek() {
        None => Ok(expr),
        Some(extra) => Err(format!("unexpected {extra:?} in `{text}`")),
    }
}

/// Read an aggregation expression: one call, `name(arg, …)`. A word
/// argument is a key the aggregation reads.
pub(super) fn parse_aggregation(text: &str) -> Result<Aggregation, String> {
    let mut parser = Parser {
        tokens: lex(text)?,
        pos: 0,
    };
    let unreadable = || format!("`{text}` is not one `name(args)` call");
    let Some(Token::Word(function)) = parser.next() else {
        return Err(unreadable());
    };
    parser.expect(Token::Open).map_err(|_| unreadable())?;
    let mut keys = Vec::new();
    if parser.peek() != Some(&Token::Close) {
        loop {
            match parser.next() {
                Some(Token::Word(key)) if !is_keyword(&key) => keys.push(key),
                Some(Token::Text(_) | Token::Number(_)) => {}
                _ => return Err(unreadable()),
            }
            if parser.peek() != Some(&Token::Comma) {
                break;
            }
            parser.pos += 1;
        }
    }
    parser.expect(Token::Close).map_err(|_| unreadable())?;
    match parser.peek() {
        None => Ok(Aggregation { function, keys }),
        Some(_) => Err(unreadable()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cmp(key: &str, op: Op, values: &[&str]) -> Expr {
        Expr::Cmp(Clause {
            key: key.into(),
            op,
            values: values.iter().map(|v| Literal::Text((*v).into())).collect(),
        })
    }

    /// The reader builds the tree the precedence rules call for, and keeps
    /// dotted keys whole.
    #[test]
    fn a_filter_parses_to_the_tree_its_precedence_implies() {
        let expr = parse_filter(
            "service.name = 'a' AND (event IN ('x', 'y') OR NOT scope_name LIKE '%z%') AND n EXISTS",
        )
        .unwrap();
        assert_eq!(
            expr,
            Expr::And(vec![
                cmp("service.name", Op::Eq, &["a"]),
                Expr::Or(vec![
                    cmp("event", Op::In, &["x", "y"]),
                    Expr::Not(Box::new(cmp(
                        "scope_name",
                        Op::Other("LIKE".into()),
                        &["%z%"]
                    ))),
                ]),
                cmp("n", Op::Exists, &[]),
            ])
        );
        let keys = |clauses: Vec<&Clause>| -> Vec<String> {
            clauses.into_iter().map(|c| c.key.clone()).collect()
        };
        assert_eq!(
            keys(expr.clauses()),
            ["service.name", "event", "scope_name", "n"]
        );
        // Only the top-level conjuncts are required; the `OR` group is not.
        assert_eq!(keys(expr.required()), ["service.name", "n"]);

        // `AND` binds tighter than `OR`: nothing here is required.
        let loose = parse_filter("a = 'x' AND b = 'y' OR c != 3").unwrap();
        assert!(matches!(&loose, Expr::Or(parts) if parts.len() == 2));
        assert!(loose.required().is_empty());
        assert_eq!(loose.clauses()[2].values, [Literal::Bare("3".into())]);
    }

    /// What the reader does not understand is an error, not an empty result.
    /// The first case is the control: the same text, well formed.
    #[test]
    fn an_unreadable_filter_is_an_error() {
        parse_filter("event = 'launcher_summary'").expect("the control parses");
        for bad in [
            "",
            "   ",
            "event = 'launcher_summary",
            "event = launcher_summary",
            "event == 'launcher_summary'",
            "event = 'a' AND",
            "event = 'a' event = 'b'",
            "(event = 'a'",
            "event = 'a')",
            "launcher_summary",
            "has(tags, 'x')",
            "event IN ['a', 'b']",
            "event IN ()",
            "n BETWEEN 1 AND 2",
            "event NOT = 'a'",
            "AND = 'a'",
            "event = 'a' ; drop",
        ] {
            assert!(parse_filter(bad).is_err(), "`{bad}` parsed");
        }
    }

    #[test]
    fn an_aggregation_is_one_call_and_names_the_keys_it_reads() {
        let agg = |function: &str, keys: &[&str]| Aggregation {
            function: function.into(),
            keys: keys.iter().map(|k| (*k).to_string()).collect(),
        };
        assert_eq!(parse_aggregation("count()"), Ok(agg("count", &[])));
        assert_eq!(parse_aggregation(" count( ) "), Ok(agg("count", &[])));
        assert_eq!(
            parse_aggregation("p95(duration_ms)"),
            Ok(agg("p95", &["duration_ms"]))
        );
        assert_eq!(
            parse_aggregation("quantile(0.5, cimmeria.session_kind)"),
            Ok(agg("quantile", &["cimmeria.session_kind"]))
        );
        for bad in [
            "",
            "count",
            "count(",
            "count())",
            "count() + 1",
            "count(), sum(n)",
            "sum(a,)",
            "(count())",
            "count(sum(n))",
        ] {
            assert!(parse_aggregation(bad).is_err(), "`{bad}` parsed");
        }
    }
}
