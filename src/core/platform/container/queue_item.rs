/*
Queue Item Container

A Queue Item is a type of Message that is placed in a Queue via the Queue Manager Service.

A Queue Item will be abstracted in the Application Layer along with an abstraction of the
Queue Manager Service. 

This containter is the base for the Queue Item Message Container on which more complex 
Queue Item Messages can be built.

Example:
A typical use case would be a Job Queue where the Queue Item is a Job that needs to be 
processed.  The Job Queue Item and Job Queue Manager Service would be defined in the
Application Layer.

*/