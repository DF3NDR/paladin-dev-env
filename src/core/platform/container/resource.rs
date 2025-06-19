/*
Resource

A Resource a platform type which uses a Node type for resources stored in the system. 
A resource could be a file that is a document, a video, a image, or any other type of content
which is defined at the application layer.

The only requirement of a type built using the Resource is that it has valid url.
It can have other fields. 

Example of a Resource type:

A resource type could be a file stored which is accessible through the file system so 
would require that it uses the standard scheme starting with `file://`.  It could be
a custom resource like a file that is stored in a configurable location and utilizing a 
custom scheme like 'private://', 'public://' or 'tmp://' or it could be a resource that is
available over the internet and uses the standard `http://` or `https://` scheme.
*/

use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::hash::Hash;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Resource<T>{
    pub uuid: Uuid,
    pub created: DateTime<Utc>,
    pub modified: DateTime<Utc>,
    pub resource: T,
}

impl <T> Resource<T> {
    pub fn new(
        resource: T, 
    ) -> Self {
        Self {
            uuid: Uuid::new_v4(),
            created: Utc::now(),
            modified: Utc::now(),
            resource,
        }
    }

}

