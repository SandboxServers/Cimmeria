"""
ContentEngine -- data-driven event / condition / action system.

Chains are loaded from the resources.content_* tables and indexed by
(event_type, event_key) for fast dispatch.  Classic scripts coexist with
the engine: each migrated script calls `engine.is_handled()` and early-
returns if the engine owns it.

Toggle levels
-------------
1. Global kill switch     : ContentEngine.ENABLED = False  (Python console)
2. Per-chain DB flag      : content_chains.enabled = false + engine.reload()
3. Classic script guard   : script.__init__ checks is_handled() and returns
"""

import Atrea
from common import Constants
from common.defs.Def import DefMgr
from common.Config import Config
from cell.Minigame import Minigame


# ---------------------------------------------------------------------------
# Internal data-transfer objects (plain dicts would also work; classes aid
# readability and allow IDE auto-complete during development).
# ---------------------------------------------------------------------------

class _ChainDef(object):
    __slots__ = ('chain_id', 'description', 'scope_type', 'scope_id',
                 'enabled', 'priority', 'triggers', 'conditions', 'actions')

    def __init__(self, row):
        self.chain_id    = int(row[0])
        self.description = row[1]
        self.scope_type  = row[2]
        self.scope_id    = int(row[3]) if row[3] is not None else None
        self.enabled     = bool(row[4])
        self.priority    = int(row[5])
        self.triggers    = []   # list of _TriggerDef
        self.conditions  = []   # list of _ConditionDef
        self.actions     = []   # list of _ActionDef


class _TriggerDef(object):
    __slots__ = ('trigger_id', 'chain_id', 'event_type', 'event_key',
                 'scope', 'once', 'sort_order')

    def __init__(self, row):
        self.trigger_id  = int(row[0])
        self.chain_id    = int(row[1])
        self.event_type  = row[2]
        self.event_key   = row[3]
        self.scope       = row[4]
        self.once        = bool(row[5])
        self.sort_order  = int(row[6])


class _ConditionDef(object):
    __slots__ = ('condition_id', 'chain_id', 'condition_type', 'target_id',
                 'target_key', 'operator', 'value', 'sort_order')

    def __init__(self, row):
        self.condition_id   = int(row[0])
        self.chain_id       = int(row[1])
        self.condition_type = row[2]
        self.target_id      = int(row[3]) if row[3] is not None else None
        self.target_key     = row[4]
        self.operator       = row[5]
        self.value          = row[6]
        self.sort_order     = int(row[7])


class _ActionDef(object):
    __slots__ = ('action_id', 'chain_id', 'action_type', 'target_id',
                 'target_key', 'params', 'delay_ms', 'sort_order')

    def __init__(self, row):
        self.action_id   = int(row[0])
        self.chain_id    = int(row[1])
        self.action_type = row[2]
        self.target_id   = int(row[3]) if row[3] is not None else None
        self.target_key  = row[4]
        # params stored as jsonb; BigWorld exposes it as a Python dict via
        # the DB layer.  Accept both dict and None.
        self.params      = row[5] if isinstance(row[5], dict) else {}
        self.delay_ms    = int(row[6])
        self.sort_order  = int(row[7])


# ---------------------------------------------------------------------------
# ContentEngine
# ---------------------------------------------------------------------------

class ContentEngine(object):
    """Singleton data-driven content engine."""

    ENABLED = True  # global kill switch (Python console: engine.ENABLED = False)

    def __init__(self):
        self.chains       = {}   # chain_id -> _ChainDef
        self.event_index  = {}   # (event_type, event_key) -> [chain_id, ...]
        self._handled     = {}   # (scope_type, scope_id) -> True
        # Per-player state: entity_id -> {'counters': {name: int}, 'subs': [sub_id]}
        self._player_state = {}

    # ------------------------------------------------------------------
    # Public API
    # ------------------------------------------------------------------

    def is_handled(self, scope_type, scope_id):
        """Return True if the engine is active for this scope."""
        return self.ENABLED and (scope_type, scope_id) in self._handled

    def load(self):
        """Query DB and build indexes.  Called once at server startup."""
        try:
            import Atrea.database as db
        except ImportError:
            # Not available in all contexts; silently skip.
            return

        try:
            chains_rows = db.query(
                "SELECT chain_id, description, scope_type, scope_id, enabled, priority "
                "FROM resources.content_chains WHERE enabled = true "
                "ORDER BY priority DESC, chain_id")
            triggers_rows = db.query(
                "SELECT trigger_id, chain_id, event_type, event_key, scope, once, sort_order "
                "FROM resources.content_triggers ORDER BY sort_order")
            conditions_rows = db.query(
                "SELECT condition_id, chain_id, condition_type, target_id, target_key, operator, value, sort_order "
                "FROM resources.content_conditions ORDER BY sort_order")
            actions_rows = db.query(
                "SELECT action_id, chain_id, action_type, target_id, target_key, params, delay_ms, sort_order "
                "FROM resources.content_actions ORDER BY sort_order")
        except Exception as e:
            Atrea.log("ContentEngine.load error: {}".format(e))
            return

        self._build_from_rows(chains_rows, triggers_rows, conditions_rows, actions_rows)

    def reload(self):
        """Hot-reload chain data and re-register all online players."""
        old_handled = dict(self._handled)
        self.chains       = {}
        self.event_index  = {}
        self._handled     = {}
        self.load()
        # Re-register players that were previously registered
        for space in Atrea.getSpaces():
            for entity in Atrea.findEntities(space.spaceId):
                if hasattr(entity, '_content_engine_registered'):
                    space_name = space.name if hasattr(space, 'name') else ''
                    self.register_player(entity, space_name)

    def register_player(self, player, space_name):
        """Subscribe all matching chains for player on zone entry."""
        if not self.ENABLED:
            return

        eid = player.entityId
        if eid in self._player_state:
            self._unregister_player(player)

        self._player_state[eid] = {'counters': {}, 'subs': []}
        player._content_engine_registered = True

        registered_scopes = set()
        for chain_id, chain in self.chains.items():
            if not chain.enabled:
                continue
            scope_matches = (
                chain.scope_type == 'global' or
                (chain.scope_type == 'space' and chain.scope_id is None) or
                (chain.scope_type == 'space' and str(chain.scope_id) == space_name) or
                chain.scope_type in ('mission', 'effect')  # always register
            )
            if not scope_matches:
                continue

            for trig in chain.triggers:
                self._subscribe_trigger(player, chain, trig)

            scope_key = (chain.scope_type, chain.scope_id)
            if chain.scope_type in ('mission', 'space', 'effect') and chain.scope_id is not None:
                registered_scopes.add(scope_key)

        for scope_key in registered_scopes:
            self._handled[scope_key] = True

    def on_effect_event(self, event_type, effect_id, effect_instance, args=None):
        """Called by EffectScript lifecycle hooks to dispatch effect events."""
        if not self.ENABLED:
            return False

        owner = effect_instance.owner
        chains = self._chains_for_event(event_type, str(effect_id))
        if not chains:
            chains = self._chains_for_event(event_type, None)
        handled = False
        for chain_id in chains:
            chain = self.chains.get(chain_id)
            if chain and chain.enabled:
                self._on_event(owner, chain_id, args or {}, effect=effect_instance)
                handled = True
        return handled

    # ------------------------------------------------------------------
    # Internal helpers
    # ------------------------------------------------------------------

    def _build_from_rows(self, chains_rows, triggers_rows, conditions_rows, actions_rows):
        for row in chains_rows:
            cd = _ChainDef(row)
            self.chains[cd.chain_id] = cd

        for row in triggers_rows:
            td = _TriggerDef(row)
            chain = self.chains.get(td.chain_id)
            if chain:
                chain.triggers.append(td)
                key = (td.event_type, td.event_key)
                if key not in self.event_index:
                    self.event_index[key] = []
                self.event_index[key].append(td.chain_id)

        for row in conditions_rows:
            cd = _ConditionDef(row)
            chain = self.chains.get(cd.chain_id)
            if chain:
                chain.conditions.append(cd)

        for row in actions_rows:
            ad = _ActionDef(row)
            chain = self.chains.get(ad.chain_id)
            if chain:
                chain.actions.append(ad)

    def _chains_for_event(self, event_type, event_key):
        ids = list(self.event_index.get((event_type, event_key), []))
        if event_key is not None:
            ids += self.event_index.get((event_type, None), [])
        return ids

    def _subscribe_trigger(self, player, chain, trig):
        """Register a single subscription with the player's event bus."""
        eid = player.entityId
        chain_id = chain.chain_id
        event_type = trig.event_type
        event_key  = trig.event_key

        # Effect events are routed via on_effect_event, not subscriptions
        if event_type.startswith('effect_'):
            return

        if event_type == 'player_loaded':
            sub_topic = 'player.loaded'
        elif event_type == 'interact_tag':
            sub_topic = 'entity.interact.tag::' + (event_key or '')
        elif event_type == 'interact_template':
            sub_topic = 'entity.interact.template::' + (event_key or '')
        elif event_type == 'entity_dead_tag':
            sub_topic = 'entity.dead.tag::' + (event_key or '')
        elif event_type in ('enter_region', 'exit_region'):
            sub_topic = 'client_hinted_region::' + (event_key or '')
        elif event_type == 'teleport_in':
            sub_topic = 'teleport::in'
        elif event_type == 'dialog_open':
            sub_topic = 'dialog.open::' + (event_key or '')
        elif event_type == 'dialog_choice':
            sub_topic = 'dialog.choice::' + (event_key or '')
        elif event_type == 'dialog_set_open':
            sub_topic = 'dialog_set.open::' + (event_key or '')
        elif event_type == 'mission_accepted':
            sub_topic = 'mission.accepted::' + (event_key or '')
        elif event_type == 'mission_completed':
            sub_topic = 'mission.completed::' + (event_key or '')
        elif event_type == 'item_use':
            sub_topic = 'item.use::' + (event_key or '')
        else:
            return  # unknown event type

        def make_callback(cid, etype, ekey):
            def cb(args):
                chain_def = self.chains.get(cid)
                if chain_def and chain_def.enabled:
                    if etype == 'enter_region' and not args.get('entering', False):
                        return False
                    if etype == 'exit_region' and args.get('entering', True):
                        return False
                    if etype == 'teleport_in':
                        expected_region = None
                        for td2 in chain_def.triggers:
                            if td2.chain_id == cid and td2.event_type == 'teleport_in':
                                expected_region = td2.event_key
                                break
                        if expected_region is not None:
                            if str(args.get('regionId', '')) != str(expected_region):
                                return False
                    self._on_event(player, cid, args)
                return False
            return cb

        cb = make_callback(chain_id, event_type, event_key)
        sub_id = player.subscribe(sub_topic, cb, once=trig.once)
        if eid in self._player_state:
            self._player_state[eid]['subs'].append(sub_id)

    def _unregister_player(self, player):
        eid = player.entityId
        state = self._player_state.pop(eid, None)
        if state:
            for sub_id in state['subs']:
                try:
                    player.unsubscribe(sub_id)
                except Exception:
                    pass

    def _on_event(self, player, chain_id, args, effect=None):
        chain = self.chains.get(chain_id)
        if chain is None or not chain.enabled:
            return

        if not self._evaluate_conditions(chain.conditions, player, args):
            return

        for action in sorted(chain.actions, key=lambda a: a.sort_order):
            if action.delay_ms > 0:
                def make_delayed(act):
                    def _run():
                        self._execute_action(act, player, args, effect)
                    return _run
                Atrea.addTimer(Atrea.getGameTime() + action.delay_ms / 1000.0,
                               make_delayed(action))
            else:
                self._execute_action(action, player, args, effect)

    # ------------------------------------------------------------------
    # Condition evaluation
    # ------------------------------------------------------------------

    def _evaluate_conditions(self, conditions, player, args):
        """All conditions must pass (AND logic)."""
        for cond in sorted(conditions, key=lambda c: c.sort_order):
            if not self._check_condition(cond, player, args):
                return False
        return True

    _MISSION_STATUS_MAP = {
        'not_active': Constants.MISSION_Not_Active,
        'active':     Constants.MISSION_Active,
        'completed':  Constants.MISSION_Completed,
        'failed':     Constants.MISSION_Failed,
    }

    def _check_condition(self, cond, player, args):
        ct  = cond.condition_type
        op  = cond.operator
        val = cond.value

        def compare(a, b):
            try:
                a_n = int(a) if not isinstance(a, int) else a
                b_n = int(b) if not isinstance(b, int) else b
            except (TypeError, ValueError):
                a_n, b_n = a, b
            if op == 'eq':  return a_n == b_n
            if op == 'neq': return a_n != b_n
            if op == 'lt':  return a_n < b_n
            if op == 'lte': return a_n <= b_n
            if op == 'gt':  return a_n > b_n
            if op == 'gte': return a_n >= b_n
            return False

        if ct == 'always':
            return True

        if ct == 'mission_status':
            status = player.missions.getMissionStatus(cond.target_id)
            mapped = self._MISSION_STATUS_MAP.get(val, val)
            return compare(status, mapped)

        if ct == 'step_status':
            step_id = int(cond.target_key) if cond.target_key else 0
            status = player.missions.getStepStatus(cond.target_id, step_id)
            mapped = self._MISSION_STATUS_MAP.get(val, val)
            return compare(status, mapped)

        if ct == 'objective_status':
            obj_id = int(cond.target_key) if cond.target_key else 0
            status = player.missions.getObjectiveStatus(cond.target_id, obj_id)
            mapped = self._MISSION_STATUS_MAP.get(val, val)
            return compare(status, mapped)

        if ct == 'has_item':
            item = player.inventory.findItem(cond.target_id, None, True)
            present = item is not None
            return present if val == 'true' else not present

        if ct == 'archetype':
            return compare(player.archetype, val)

        if ct == 'entering':
            return compare(args.get('entering', False), val == 'true')

        if ct == 'level':
            return compare(getattr(player, 'level', 0), val)

        if ct == 'counter':
            eid = player.entityId
            name = cond.target_key or ''
            current = self._player_state.get(eid, {}).get('counters', {}).get(name, 0)
            return compare(current, val)

        return True  # unknown condition type: pass through

    # ------------------------------------------------------------------
    # Action execution
    # ------------------------------------------------------------------

    def _execute_action(self, action, player, args, effect=None):
        at  = action.action_type
        tid = action.target_id
        tkey = action.target_key
        p   = action.params

        # ---- Mission actions ----
        if at == 'accept_mission':
            player.missions.accept(tid)
        elif at == 'complete_mission':
            player.missions.complete(tid)
        elif at == 'advance_step':
            step_id = int(tkey) if tkey else p.get('step_id')
            if step_id:
                player.missions.advance(tid, int(step_id))
        elif at == 'complete_objective':
            obj_id = int(tkey) if tkey else p.get('objective_id')
            if obj_id:
                player.missions.completeObjective(tid, int(obj_id))
        elif at == 'fail_mission':
            player.missions.fail(tid)
        elif at == 'fail_objective':
            obj_id = int(tkey) if tkey else p.get('objective_id')
            if obj_id:
                player.missions.failObjective(tid, int(obj_id))
        elif at == 'abandon_mission':
            player.missions.abandon(tid)
        elif at == 'display_rewards':
            player.missions.displayRewards(tid)

        # ---- Inventory actions ----
        elif at == 'add_item':
            container = p.get('container', 0)
            qty = p.get('qty', 1)
            if container:
                player.inventory.addItem(int(container), tid, int(qty))
            else:
                player.inventory.pickedUpItem(tid, int(qty))
            player.inventory.flushUpdates()
        elif at == 'remove_item':
            qty = p.get('qty', 1)
            player.inventory.removeItemByDesign(tid, int(qty), False)
            player.inventory.flushUpdates()

        # ---- Dialog actions ----
        elif at == 'display_dialog':
            npc = None
            if tkey:
                npc = self._find_entity_by_tag(player, tkey)
            player.displayDialog(npc, tid)
        elif at == 'add_dialog_set':
            slot = p.get('slot', 0)
            dialogSet = DefMgr.get('interaction_set_map', tid)
            if dialogSet is not None:
                player.addDialog(int(slot), dialogSet, p.get('mission_id') or None)
        elif at == 'remove_dialog_set':
            slot = p.get('slot', 0)
            dialogSet = DefMgr.get('interaction_set_map', tid)
            if dialogSet is not None:
                player.removeDialog(int(slot), dialogSet)

        # ---- Entity actions ----
        elif at == 'set_interaction_type':
            entity = self._find_entity_by_tag(player, tkey)
            if entity:
                mask = p.get('mask', tkey)
                op2 = p.get('op', '|')
                if op2 == '|':
                    entity.setInteractionType(entity.interactionType | int(mask))
                elif op2 == '~':
                    entity.setInteractionType(entity.interactionType & ~int(mask))
                else:
                    entity.setInteractionType(int(mask))
        elif at == 'set_aggression':
            entity = self._find_entity_by_tag(player, tkey)
            if entity:
                entity.setAggression(int(p.get('level', tid or 0)))
        elif at == 'set_visible':
            entity = self._find_entity_by_tag(player, tkey)
            visible = p.get('visible', True)
            if entity:
                entity.setVisible(bool(visible))
        elif at == 'move_entity':
            entity = self._find_entity_by_tag(player, tkey)
            if entity:
                dest = p.get('destination')
                world = p.get('world', '') or None
                if dest:
                    parts = [float(x) for x in str(dest).split(',')]
                    entity.moveTo(parts[0], parts[1], parts[2], worldName=world)
        elif at == 'spawn_entity':
            tpl = DefMgr.get('template', tid)
            loc_str = p.get('location', '0,0,0')
            parts = [float(x) for x in str(loc_str).split(',')]
            if tpl:
                player.space.createEntity(tpl, Atrea.Vector3(parts[0], parts[1], parts[2]))
        elif at == 'destroy_entity':
            entity = self._find_entity_by_tag(player, tkey)
            if entity:
                Atrea.destroyCellEntity(entity.entityId)
        elif at == 'add_waypoint':
            entity = self._find_entity_by_tag(player, tkey)
            if entity:
                pos_str = p.get('position', '0,0,0')
                parts = [float(x) for x in str(pos_str).split(',')]
                entity.addWaypoint(Atrea.Vector3(parts[0], parts[1], parts[2]), lambda: None)

        # ---- Presentation actions ----
        elif at == 'play_sequence':
            seq_id = tid or int(p.get('sequence_id', 0))
            player.playSequence(seq_id, player.entityId, True, 0,
                                viewType=0, instanceId=0,
                                playOnSelf=True, playOnWitnesses=True)
        elif at == 'system_message':
            player.client.onSystemCommunication(11, tid, '', [])
        elif at == 'trigger_transporter':
            region_id = int(p.get('regionId', tid or 0))
            player.triggerTransporter(region_id)

        # ---- Minigame actions ----
        elif at == 'start_minigame':
            self._execute_minigame_chain(action, player)

        # ---- Counter actions ----
        elif at == 'increment_counter':
            eid = player.entityId
            name = tkey or ''
            state = self._player_state.setdefault(eid, {'counters': {}, 'subs': []})
            state['counters'][name] = state['counters'].get(name, 0) + int(p.get('amount', 1))
        elif at == 'reset_counter':
            eid = player.entityId
            name = tkey or ''
            state = self._player_state.get(eid, {})
            if 'counters' in state:
                state['counters'][name] = 0

        # ---- Effect actions ----
        elif at == 'qr_combat_damage':
            if effect:
                stat_id   = int(p.get('stat_id', tid or 7))
                source_id = int(p.get('source_id', 15))
                amount    = int(p.get('amount', 0))
                effect.qrCombatDamage(stat_id, source_id, amount, True, True)
        elif at == 'change_stat':
            if effect:
                stat_id = int(p.get('stat_id', tid or 8))
                min_val = int(p.get('min', 0))
                max_val = int(p.get('max', 100))
                effect.changeStat(stat_id, min_val, max_val, False)
        elif at == 'apply_effect':
            eff_def = DefMgr.get('effect', tid)
            if eff_def:
                player.abilities.addEffect(eff_def, player.entityId)
        elif at == 'remove_effect':
            eff_def = DefMgr.get('effect', tid)
            if eff_def:
                player.abilities.removeEffect(eff_def)

    def _execute_minigame_chain(self, action, player):
        """Start a minigame; on victory execute chains listed in params.on_victory_chains."""
        game_name = action.target_key or action.params.get('game_name', 'Livewire')
        on_victory_chains = action.params.get('on_victory_chains', [])

        gameIds = [k for k, v in Config.MINIGAME_NAMES.items() if v == game_name]
        if not gameIds:
            return
        game_id = gameIds[0]

        def mg_result(mg, rc):
            if rc == Constants.MINIGAME_RESULT_Victory:
                for cid in on_victory_chains:
                    chain = self.chains.get(int(cid))
                    if chain and chain.enabled:
                        self._on_event(player, int(cid), {})

        minigame = Minigame('', 1, game_id, 1, 0xffff,
                            lambda mg, rc: mg_result(mg, rc))
        player.requestStartMinigame(minigame)

    def _find_entity_by_tag(self, player, tag):
        if not tag:
            return None
        for entity in Atrea.findEntities(player.space.spaceId):
            if entity.tag == tag:
                return entity
        return None


# ---------------------------------------------------------------------------
# Module-level singleton
# ---------------------------------------------------------------------------
engine = ContentEngine()
