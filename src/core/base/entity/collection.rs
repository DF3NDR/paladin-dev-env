/*
Collection 

A Collection is a generic type that can be used to store a collection of Data Items.
*/

use serde::{Deserialize, Serialize};
// DataItem type is a generic type that can be used to store any type of data.
// The data_points field is a vector of DataPoint instances
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Item<T> {
    pub item: T,
}

impl<T> Item<T> {
    pub fn new(item: T) -> Self {
        Self { item }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CollectionType<T> {
    pub uuid: Uuid,
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