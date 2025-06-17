/*
Logger Port

A port that defines how the application logs information. This could be a file, a database,
a logging service, or any other logging mechanism.

This port is responsible for providing an abstraction layer that allows the application to log
information without being tightly coupled to the details of how that information is logged.

Typical implementations of this port would be for the adapter to translate log messages
from the application into calls to the logging mechanism, and to translate log messages
from the logging mechanism back into a format that the application can use.

An example of a logging mechanism would be a file logger that writes log messages to a file.
The Logger Port would provide an interface for the application to log messages, and the adapter
would translate those messages into calls to the file logger.

Errors and Events emitted by the application would typically be made available to this port
by way of teh application's use cases through the logging_service.
*/