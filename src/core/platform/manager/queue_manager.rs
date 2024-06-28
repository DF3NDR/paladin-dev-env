/*
Queue Manager Service

This is the Queue Manager Service, it is responsible for managing the queue. This module 
contains the Queue Manager Service, its related traits and implementations.

Within our Hexagonal Architecture, the Queue Manager Service is at the Service Layer, 
above the Domain Layer and below the Application layer. It places Queue Item Containers 
into a Queue and retrieves them from a Queue.   

The Queue is a FIFO (First In First Out) data structure that is used to store Queue Items.
The Queue Service is abstracted in the application layer so that multiple Queues may exist.

Ports for the External Queue Adapters are also on the Application Layer, 

Because we are using a Hexagonal Architecture the Queue  Application Layer


*/