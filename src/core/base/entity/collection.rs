/*
Collection 

A Collection is a generic type that can be used to store a collection of items.

*/

// DataItem type is a generic type that can be used to store any type of data.
// The data_points field is a vector of DataPoint instances
use uuid::Uuid;

pub struct Item<T> {
    item: T,
}

pub struct CollectionType<T> {
    uuid: Uuid,
    items: Vec<Item<T>>,
}
