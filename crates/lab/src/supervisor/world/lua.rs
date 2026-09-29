//! Lua chunks the world tools run, and their parsers. Every chunk *reads*
//! except [`target_unit_chunk`] (the N3 fallback, reported as such) and
//! the PreRender re-subscription in [`projection_request_chunk`], which
//! only adds a reader in front of a stock handler.
//!
//! **Projection.** The stock UI projects world points with
//! `view:worldToPixel(pos)` inside `Events.PreRender` handlers
//! (`SCTFrame.lua`); `view` exists only during that call. The lab
//! re-subscribes a stock HUD window's PreRender to `LabWorld.pre`, which
//! calls the module's own handler first (so the HUD keeps working) and then
//! projects the points the supervisor queued in `LabWorld.req`. A window
//! holds one subscription per event (`Options.lua` unsubscribes by event
//! alone), which is why the lab chains instead of adding a second one.

use super::geometry::{Screen, Vec3};
use crate::supervisor::flows::widgets::lua_quote;

/// Stock `(window, module)` pairs whose PreRender the lab can chain, in
/// order of preference. SCT is loaded for the whole in-world session.
pub const PROJECTION_HOSTS: [(&str, &str); 2] =
    [("SCTWin", "SCTMod"), ("MinimapWin", "MinimapMod")];

/// `Unit.Player, Unit.Target, Unit.MouseOver` as numbers.
pub const UNIT_CONSTANTS_CHUNK: &str =
    "return tostring(Unit.Player), tostring(Unit.Target), tostring(Unit.MouseOver)";

/// The stock unit-slot numbers the tools use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnitSlots {
    pub player: i32,
    pub target: i32,
    pub mouse_over: i32,
}

pub fn parse_unit_constants(r: &[String]) -> Result<UnitSlots, String> {
    let n = |i: usize, what: &str| -> Result<i32, String> {
        r.get(i)
            .and_then(|s| s.parse::<f64>().ok())
            .map(|v| v as i32)
            .ok_or_else(|| format!("Unit.{what} is not a number ({r:?})"))
    };
    Ok(UnitSlots {
        player: n(0, "Player")?,
        target: n(1, "Target")?,
        mouse_over: n(2, "MouseOver")?,
    })
}

/// What the stock unit API says about one (pinned) slot.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct UnitInfo {
    pub slot: i32,
    pub exists: bool,
    pub name: String,
    pub level: Option<i64>,
    /// `AggressionLevel` key name (`Friendly`, `Hostile`, ...) or the raw value.
    pub hostility: String,
    pub is_friend: Option<bool>,
    pub mob_id: Option<i64>,
}

/// One line per slot: `slot, exists, name, level, hostility, friend, mob id`,
/// tab-separated. Every call is pcall'd; a failed field is empty.
pub fn unit_info_chunk(slots: &[i32]) -> String {
    let list = slots
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "local agg = {{}} \
         if AggressionLevel then for k, v in pairs(AggressionLevel) do agg[v] = k end end \
         local function g(f, s) if type(f) ~= 'function' then return '' end \
           local ok, v = pcall(f, s) if ok and v ~= nil then return tostring(v) end return '' end \
         local out = {{}} \
         for _, s in ipairs({{{list}}}) do \
           local h = '' local okh, hv = pcall(unitHostilityToPlayer, s) \
           if okh and hv ~= nil then h = agg[hv] or tostring(hv) end \
           local name = g(unitName, s):gsub('[\\t\\n]', ' ') \
           out[#out + 1] = table.concat({{ s, g(unitExists, s), name, g(unitLevel, s), h, \
             g(unitIsFriend, s), g(unitMobId, s) }}, '\\t') \
         end \
         return table.concat(out, '\\n')"
    )
}

pub fn parse_unit_info(r: &[String]) -> Vec<UnitInfo> {
    let Some(text) = r.first() else {
        return Vec::new();
    };
    text.lines()
        .filter_map(|line| {
            let f: Vec<&str> = line.split('\t').collect();
            let slot = f.first()?.parse::<f64>().ok()? as i32;
            let get = |i: usize| f.get(i).copied().unwrap_or("");
            let int = |s: &str| s.parse::<f64>().ok().map(|v| v as i64);
            let boolean = |s: &str| match s {
                "true" => Some(true),
                "false" => Some(false),
                _ => None,
            };
            Some(UnitInfo {
                slot,
                exists: get(1) == "true",
                name: get(2).to_string(),
                level: int(get(3)),
                hostility: get(4).to_string(),
                is_friend: boolean(get(5)),
                mob_id: int(get(6)),
            })
        })
        .collect()
}

/// Queue `points` (client coordinates) for projection on the next frame,
/// installing (or, with `force`, re-installing) the PreRender chain first.
/// Returns `reqid, frame, host, root width, root height`, or `'nohost'`.
pub fn projection_request_chunk(points: &[Vec3], force: bool) -> String {
    let hosts = PROJECTION_HOSTS
        .iter()
        .map(|(w, m)| format!("{{{}, {}}}", lua_quote(w), lua_quote(m)))
        .collect::<Vec<_>>()
        .join(", ");
    let pts = points
        .iter()
        .map(|p| format!("{{{:.2}, {:.2}, {:.2}}}", p.x, p.y, p.z))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "LabWorld = LabWorld or {{}} local LW = LabWorld \
         if not LW.pre then \
           LW.frame = 0 \
           function LW.pre(window, view) \
             if LW.orig then pcall(LW.orig, window, view) end \
             LW.frame = (LW.frame or 0) + 1 \
             local req = LW.req \
             if req and LW.done ~= LW.reqid then \
               local out = {{}} \
               for i, p in ipairs(req) do \
                 local ok, s, hit = pcall(view.worldToPixel, view, Vector3(p[1], p[2], p[3])) \
                 if ok and hit and s then out[i] = string.format('%.1f,%.1f', s.x, s.y) \
                 elseif ok then out[i] = '-' else out[i] = '!' .. tostring(s) end \
               end \
               LW.res = table.concat(out, ';') LW.done = LW.reqid \
             end \
           end \
         end \
         if {force} or not LW.host then \
           LW.host = nil \
           for _, h in ipairs({{ {hosts} }}) do \
             local w, m = _G[h[1]], _G[h[2]] \
             if w and m and type(m.onPreRender) == 'function' then \
               LW.orig = m.onPreRender \
               w:unsubscribe(Events.PreRender) \
               w:subscribe(Events.PreRender, 'LabWorld.pre') \
               LW.host = h[1] break \
             end \
           end \
         end \
         if not LW.host then return 'nohost' end \
         LW.reqid = (LW.reqid or 0) + 1 \
         LW.req = {{ {pts} }} \
         local r = getWindow('Root'):getPixelSize() \
         return tostring(LW.reqid), tostring(LW.frame), LW.host, tostring(r.width), tostring(r.height)"
    )
}

/// A queued projection.
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectionTicket {
    pub reqid: u64,
    pub frame: u64,
    pub host: String,
    pub screen: Screen,
}

pub fn parse_projection_ticket(r: &[String]) -> Result<ProjectionTicket, String> {
    if r.first().map(String::as_str) == Some("nohost") {
        return Err(format!(
            "no PreRender host to chain: none of {:?} is loaded (not in the world?)",
            PROJECTION_HOSTS.iter().map(|h| h.0).collect::<Vec<_>>()
        ));
    }
    let num = |i: usize| -> Result<f64, String> {
        r.get(i)
            .and_then(|s| s.parse::<f64>().ok())
            .ok_or_else(|| format!("projection request: bad results {r:?}"))
    };
    Ok(ProjectionTicket {
        reqid: num(0)? as u64,
        frame: num(1)? as u64,
        host: r.get(2).cloned().unwrap_or_default(),
        screen: Screen {
            w: num(3)?,
            h: num(4)?,
        },
    })
}

/// Has request `reqid` been projected? Returns `done, results, frame`.
pub fn projection_read_chunk(reqid: u64) -> String {
    format!(
        "local LW = LabWorld or {{}} \
         return tostring(LW.done == {reqid}), LW.res or '', tostring(LW.frame or 0)"
    )
}

/// A read of a queued projection.
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectionRead {
    pub done: bool,
    /// One entry per queued point: the pixel, or `None` when the point is
    /// behind the camera (or the call failed).
    pub points: Vec<Option<(f64, f64)>>,
    pub errors: Vec<String>,
    pub frame: u64,
}

pub fn parse_projection_read(r: &[String]) -> ProjectionRead {
    let done = r.first().map(String::as_str) == Some("true");
    let mut points = Vec::new();
    let mut errors = Vec::new();
    if done {
        for item in r.get(1).map(String::as_str).unwrap_or("").split(';') {
            if let Some(err) = item.strip_prefix('!') {
                errors.push(err.to_string());
                points.push(None);
                continue;
            }
            let p = item.split_once(',').and_then(|(x, y)| {
                Some((x.trim().parse::<f64>().ok()?, y.trim().parse::<f64>().ok()?))
            });
            points.push(p);
        }
    }
    ProjectionRead {
        done,
        points,
        errors,
        frame: r.get(2).and_then(|s| s.parse().ok()).unwrap_or(0),
    }
}

/// Visible top-level windows (children of the CEGUI root), comma-joined.
pub const VISIBLE_WINDOWS_CHUNK: &str = "local root = getWindow('Root') local out = {} \
     for i = 0, root:getChildCount() - 1 do local w = root:getChildAtIdx(i) \
       if w:isVisible() then out[#out + 1] = w:getName() end end \
     return table.concat(out, ',')";

pub fn parse_window_list(r: &[String]) -> Vec<String> {
    r.first()
        .map(|s| {
            s.split(',')
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// The stock UI's own targeting call on a pinned slot (the N3 fallback).
pub fn target_unit_chunk(slot: i32) -> String {
    format!("targetUnit({slot}) return 'ok'")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn unit_constants_parse_lua_numbers() {
        let u = parse_unit_constants(&s(&["0", "1", "14"])).unwrap();
        assert_eq!(
            u,
            UnitSlots {
                player: 0,
                target: 1,
                mouse_over: 14
            }
        );
        assert!(parse_unit_constants(&s(&["nil", "1", "2"])).is_err());
    }

    #[test]
    fn unit_info_parses_each_slot_line() {
        let text = "7700\ttrue\tCol. Marsh\t12\tFriendly\ttrue\t1234\n7701\tfalse\t\t\t\t\t";
        let rows = parse_unit_info(&s(&[text]));
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].name, "Col. Marsh");
        assert_eq!(rows[0].level, Some(12));
        assert_eq!(rows[0].hostility, "Friendly");
        assert_eq!(rows[0].is_friend, Some(true));
        assert_eq!(rows[0].mob_id, Some(1234));
        assert!(!rows[1].exists);
        assert_eq!(rows[1].level, None);
    }

    #[test]
    fn unit_info_chunk_lists_the_slots() {
        let c = unit_info_chunk(&[7700, 7701]);
        assert!(c.contains("ipairs({7700,7701})"));
        assert!(c.contains("unitHostilityToPlayer"));
    }

    #[test]
    fn projection_request_embeds_points_and_hosts() {
        let c = projection_request_chunk(&[Vec3::new(1.0, 2.5, -3.0)], false);
        assert!(c.contains("{1.00, 2.50, -3.00}"));
        assert!(c.contains("\"SCTWin\""));
        assert!(c.contains("'LabWorld.pre'"));
        assert!(c.contains("if false or not LW.host"));
        assert!(projection_request_chunk(&[], true).contains("if true or not LW.host"));
    }

    #[test]
    fn projection_ticket_parses_or_names_the_missing_host() {
        let t = parse_projection_ticket(&s(&["3", "120", "SCTWin", "1024", "768"])).unwrap();
        assert_eq!(t.reqid, 3);
        assert_eq!(
            t.screen,
            Screen {
                w: 1024.0,
                h: 768.0
            }
        );
        let e = parse_projection_ticket(&s(&["nohost"])).unwrap_err();
        assert!(e.contains("SCTWin"));
    }

    #[test]
    fn projection_read_splits_hits_misses_and_errors() {
        let r = parse_projection_read(&s(&["true", "512.0,300.5;-;!bad self", "99"]));
        assert!(r.done);
        assert_eq!(r.points, vec![Some((512.0, 300.5)), None, None]);
        assert_eq!(r.errors, vec!["bad self".to_string()]);
        assert_eq!(r.frame, 99);
        let pending = parse_projection_read(&s(&["false", "", "98"]));
        assert!(!pending.done && pending.points.is_empty());
    }
}
