use uuid::Uuid;

/// A single piece of data, a single value, held as a child of another `Entity`.
///
/// A `Field` is a base type of `Entity`. It is a generic type for building specialized
/// fields. For an entity type that is a user, a field could be a username, a password, an
/// email, and so on.
///
/// Unfinished scaffolding of the content / general-data orchestration system, retained
/// pending the ADR-0060 (g) review. The decision is recorded in `55-BASE-LAYER-REVIEW.md`
/// (keep). Nothing outside `base` consumes it; the only user is
/// [`FieldVersionService`](crate::base::service::field_version_service::FieldVersionService).
#[derive(Debug, Clone)]
pub struct Field<T> {
    /// The id of this field.
    pub fid: Uuid,
    /// The id of the parent entity. A retained entity-sense name (ADR-0060 (g)), not a
    /// WarGraph / Campaign vertex.
    pub nid: Uuid,
    /// The payload this field holds.
    pub value: T,
}

impl<T> Field<T> {
    /// Create a field around `value`.
    ///
    /// Assigns a fresh `fid` and a fresh `nid`. That `nid` refers to no parent entity; set
    /// `nid` to link the field to one.
    pub fn new(value: T) -> Self {
        Self {
            fid: Uuid::new_v4(),
            nid: Uuid::new_v4(),
            value,
        }
    }
}
