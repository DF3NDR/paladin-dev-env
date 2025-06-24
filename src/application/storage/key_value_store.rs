/*

Key Value Store Port

A key value store is a port for the storage mechanisms that store simple key value
pairs.

This key value store is an in-memory store that typically stores values in a HashMap. This
store is not persistent and will lose all data when the application is restarted unless otherwise specified.

This store is used for use cases where a temporary storage mechanism is needed, such as caching
'values' that need to be repeatedly accessed or queueing items.
*/