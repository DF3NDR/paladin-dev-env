use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// One member of a [`CollectionType`], wrapping a payload of type `T`.
///
/// Unfinished scaffolding of the content / general-data orchestration system, retained
/// pending the ADR-0060 (g) review; the decision is recorded in `55-BASE-LAYER-REVIEW.md`
/// (keep).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Item<T> {
    /// The payload of this item.
    pub item: T,
}

impl<T> Item<T> {
    pub fn new(item: T) -> Self {
        Self { item }
    }
}

/// A generic collection of [`Item`]s, identified by a `uuid`.
///
/// Unfinished scaffolding of the content / general-data orchestration system, retained
/// pending the ADR-0060 (g) review; the decision is recorded in `55-BASE-LAYER-REVIEW.md`
/// (keep). The only user is
/// [`CollectionVersionService`](crate::base::service::collection_versioning_service::CollectionVersionService).
#[derive(Debug, Clone, PartialEq)]
pub struct CollectionType<T> {
    /// The id of this collection.
    pub uuid: Uuid,
    /// The items it holds, in order.
    pub items: Vec<Item<T>>,
}

impl<T> CollectionType<T> {
    pub fn new(uuid: Uuid, items: Vec<Item<T>>) -> Self {
        Self { uuid, items }
    }

    pub fn with_uuid(uuid: Uuid) -> Self {
        Self {
            uuid,
            items: Vec::new(),
        }
    }

    pub fn items(&self) -> &[Item<T>] {
        &self.items
    }

    pub fn items_mut(&mut self) -> &mut Vec<Item<T>> {
        &mut self.items
    }

    pub fn add_item(&mut self, item: Item<T>) {
        self.items.push(item);
    }

    pub fn remove_item(&mut self, index: usize) -> Option<Item<T>> {
        if index < self.items.len() {
            Some(self.items.remove(index))
        } else {
            None
        }
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

impl<T> Default for CollectionType<T> {
    fn default() -> Self {
        Self {
            uuid: Uuid::new_v4(),
            items: Vec::new(),
        }
    }
}
