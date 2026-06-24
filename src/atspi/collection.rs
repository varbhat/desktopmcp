//! AT-SPI Collection interface.
//!
//! The Collection interface provides powerful server-side search using
//! structured match rules — much faster than client-side tree walking
//! for large UIs.
//!
//! # Match Rule structure: `(aiia{ss}iaiiasib)`
//!
//! Fields:
//!   - `ai`     states (array of u32 state bitmasks, 2 elements)
//!   - `i`      states_match_type (0=ANY, 1=ALL, 2=NONE, 3=EMPTY)
//!   - `a{ss}`  attributes dict
//!   - `i`      attributes_match_type
//!   - `a`      [roles as array of i32]  (empty = any)
//!   - `i`      roles_match_type (but this field is actually `ii` — role_count + role_match_type)
//!   - `as`     interfaces (array of interface name strings)
//!   - `i`      interfaces_match_type
//!   - `b`      invert (negate the match)
//!
//! # SortOrder: `u`
//!   0 = INVALID, 1 = CANONICAL, 2 = FLOW, 3 = TAB, 4 = REVERSE_CANONICAL,
//!   5 = REVERSE_FLOW, 6 = REVERSE_TAB
//!
//! # TreeTraversalType: `u`
//!   0 = RESTRICT_CHILDREN, 1 = RESTRICT_SIBLING, 2 = INORDER

#![allow(dead_code)]

use anyhow::{Context, Result};
use std::collections::HashMap;
use zbus::Connection;
use zvariant::OwnedObjectPath;

use super::types::{ElementId, ObjectRef};

/// A Collection match rule.
///
/// All fields are optional. Unset fields mean "any" (no filter applied).
#[derive(Debug, Default)]
pub struct MatchRule {
    /// State words to match (2 u32 bitmasks). Empty = any.
    pub states: Vec<u32>,
    /// How to match states: 0=ANY, 1=ALL, 2=NONE, 3=EMPTY.
    pub states_match_type: i32,
    /// Attribute key-value pairs to match. Empty = any.
    pub attributes: HashMap<String, String>,
    /// How to match attributes: 0=ANY, 1=ALL, 2=NONE, 3=EMPTY.
    pub attributes_match_type: i32,
    /// Role numbers to match. Empty = any.
    pub roles: Vec<i32>,
    /// Role count (same as roles.len(), or 0 for any).
    pub roles_match_type: i32,
    /// Interface names to match (e.g. "org.a11y.atspi.Action"). Empty = any.
    pub interfaces: Vec<String>,
    /// How to match interfaces: 0=ANY, 1=ALL, 2=NONE, 3=EMPTY.
    pub interfaces_match_type: i32,
    /// Invert the match result.
    pub invert: bool,
}

/// Deserialise an `a(so)` reply into `Vec<ObjectRef>`.
fn parse_refs(pairs: Vec<(String, OwnedObjectPath)>) -> Vec<ObjectRef> {
    pairs
        .into_iter()
        .map(|(bus, path)| ObjectRef { bus_name: bus, path: path.to_string() })
        .collect()
}

/// Encode a `MatchRule` into the AT-SPI D-Bus wire format `(aiia{ss}iaiiasib)`.
fn encode_match_rule(
    rule: &MatchRule,
) -> (
    Vec<u32>,
    i32,
    HashMap<String, String>,
    i32,
    Vec<i32>,
    i32,
    Vec<String>,
    i32,
    bool,
) {
    (
        if rule.states.is_empty() { vec![0u32, 0u32] } else { rule.states.clone() },
        rule.states_match_type,
        rule.attributes.clone(),
        rule.attributes_match_type,
        rule.roles.clone(),
        rule.roles_match_type,
        rule.interfaces.clone(),
        rule.interfaces_match_type,
        rule.invert,
    )
}

/// Search for elements matching a match rule, starting from the given element.
///
/// `sort_order`: 1 = CANONICAL (document order, recommended)
/// `count`: maximum results (0 = unlimited)
/// `traverse`: whether to search into subtrees of matched elements
pub async fn get_matches(
    conn: &Connection,
    id: &ElementId,
    rule: &MatchRule,
    sort_order: u32,
    count: i32,
    traverse: bool,
) -> Result<Vec<ObjectRef>> {
    let (bus, path) = id.parts()?;
    let encoded = encode_match_rule(rule);

    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.Collection"),
            "GetMatches",
            &(encoded, sort_order, count, traverse),
        )
        .await
        .context("Collection.GetMatches")?;

    let pairs: Vec<(String, OwnedObjectPath)> =
        reply.body().deserialize().context("deserialize GetMatches")?;
    Ok(parse_refs(pairs))
}

/// Search for elements matching a rule, forward from `current_object`.
pub async fn get_matches_from(
    conn: &Connection,
    id: &ElementId,
    current_object_path: &str,
    rule: &MatchRule,
    sort_order: u32,
    tree: u32,
    count: i32,
    traverse: bool,
) -> Result<Vec<ObjectRef>> {
    let (bus, path) = id.parts()?;
    let encoded = encode_match_rule(rule);

    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.Collection"),
            "GetMatchesFrom",
            &(
                current_object_path,
                encoded,
                sort_order,
                tree,
                count,
                traverse,
            ),
        )
        .await
        .context("Collection.GetMatchesFrom")?;

    let pairs: Vec<(String, OwnedObjectPath)> =
        reply.body().deserialize().context("deserialize GetMatchesFrom")?;
    Ok(parse_refs(pairs))
}

/// Get the active descendant of a collection (e.g. the focused item in a list).
pub async fn get_active_descendant(
    conn: &Connection,
    id: &ElementId,
) -> Result<ObjectRef> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.Collection"),
            "GetActiveDescendant",
            &(),
        )
        .await
        .context("Collection.GetActiveDescendant")?;
    let (bus_name, obj_path): (String, OwnedObjectPath) =
        reply.body().deserialize().context("deserialize GetActiveDescendant")?;
    Ok(ObjectRef { bus_name, path: obj_path.to_string() })
}

/// Convenience: search an application by role (u32) with CANONICAL sort.
///
/// Returns at most `count` matching element IDs.
pub async fn find_by_role(
    conn: &Connection,
    id: &ElementId,
    role: u32,
    count: i32,
) -> Result<Vec<ElementId>> {
    let rule = MatchRule {
        roles: vec![role as i32],
        roles_match_type: 1, // ALL
        ..Default::default()
    };
    let refs = get_matches(conn, id, &rule, 1, count, true).await?;
    Ok(refs.into_iter().map(|r| r.to_element_id()).collect())
}

/// Search for elements matching a rule, backwards from `current_object`.
///
/// `tree`: 0=RESTRICT_CHILDREN, 1=RESTRICT_SIBLING, 2=INORDER
/// `limit_scope`: if true, don't cross scope boundary.
pub async fn get_matches_to(
    conn: &Connection,
    id: &ElementId,
    current_object_path: &str,
    rule: &MatchRule,
    sort_order: u32,
    tree: u32,
    limit_scope: bool,
    count: i32,
    traverse: bool,
) -> Result<Vec<ObjectRef>> {
    let (bus, path) = id.parts()?;
    let encoded = encode_match_rule(rule);

    let reply = conn
        .call_method(
            Some(bus), path,
            Some("org.a11y.atspi.Collection"),
            "GetMatchesTo",
            &(current_object_path, encoded, sort_order, tree, limit_scope, count, traverse),
        )
        .await
        .context("Collection.GetMatchesTo")?;

    let pairs: Vec<(String, OwnedObjectPath)> =
        reply.body().deserialize().context("deserialize GetMatchesTo")?;
    Ok(parse_refs(pairs))
}
