/*
User Entity

The User Container is a type of Node that has a UUID with a username field. User details 
are not part of this container.  They are abstracted to the application layer. By 
defining only the most fundamental requirements within the Core we allow the application
to handle all the details that it requires while still enabling the other containers and
services at this layer to meet their requirements.  The messaging, node 
and collection management, events, jobs, tasks, etc all require either a user (or a 
transaction or transport service).

*/