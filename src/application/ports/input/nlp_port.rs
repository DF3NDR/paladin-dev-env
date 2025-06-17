/* 
NLP port

A port that defines how the application interacts with the NLP (Natural Language Processing) model.

This port is responsible for translating high-level application use cases into interactions
with the NLP model. It provides an abstraction layer that allows the application to interact with
the NLP model without being tightly coupled to its implementation details.

Typical implementations of this port would be for the adapter to translate the requirements
of the high-level use cases into calls to the NLP model, and to translate the results of those calls
back into a format that the application can use.

An NLP Api usually requires a few standard fields to be present in the request and response
like the text, model, etc. The NLP Port should handle these fields and provide a
clean interface for the application to interact with the NLP.

*/