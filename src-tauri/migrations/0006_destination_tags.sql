-- EPIC 6.3 (jamais fait, raccordé PLAN.md §6decies) : persiste les tags posés par l'utilisateur
-- sur une destination (`commands::destinations::tag_destination`) — `destinations` n'a pas de
-- table dédiée (dérivée de `flows`), donc le tag vit à part, fusionné dans
-- `storage::aggregates::get_destination_aggregated`/`list_destinations_aggregated`.

CREATE TABLE destination_tags (
    domain TEXT PRIMARY KEY,
    tag TEXT NOT NULL
);
