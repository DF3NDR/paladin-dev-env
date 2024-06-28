/*
Node

A Node is the most fundamental type in the Core. It is a type that is used to build most 
other types. It is a type that is created with  UUID, a created timestamp, a modified 
timestamp, and a generic type that for the node itself. It also has a name and version, a
boolean value for versioning.

The versioning service is automatically called whenever a Node is created or modified as 
long as the version value is `true`. The versioning service will create a new version of
the Node and store it in the repository. The versioning service will also store the
previous version of the Node in the repository.

*/
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Node<T> {
    pub uuid: Uuid,
    pub created: DateTime<Utc>,
    pub modified: DateTime<Utc>,
    pub node: T,
    pub name: Option<String>,
    pub version: bool,
}

impl<T> Node<T> {
    // Being a Domain Entity in Hexagonal Architecture the NodeItem should not have any knowledge of the repository
    pub fn new(
        node: T, 
        name: Option<String>,
    ) -> Self {
        Self {
            uuid: Uuid::new_v4(),
            created: Utc::now(),
            modified: Utc::now(),
            node,
            name: None,
            version: true,
        }
    }
}