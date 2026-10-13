/*
Field

A Field is a base type of Entity that is used to store a single piece of data, a single value,
which as a child to another Entity.

The Field is a generic type for building specialized Fields.

Example:
For an Entity type that is a User, a Field could be a username, a password, an email, etc.
*/
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Field<T> {
    pub fid: Uuid,
    pub nid: Uuid,
    pub value: T,
}

impl<T> Field<T> {
    pub fn new(value: T) -> Self {
        Self {
            fid: Uuid::new_v4(),
            nid: Uuid::new_v4(),
            value,
        }
    }
}
