/*
Element Component

An Element Component is a type that represents a (potential) reference connecting 
entities. It doesn't contain any link to an Entity type, rather a higher order Entity 
type that can use an Element type and this can connect it to other entities.  It might 
also be be thought of as an "common/shared reference" or a "shared component" or a 
"potential edge". 

The Element Component type can be abstracted at higher layers to provide the references
in complex relationship structures.

These can be one-to-one, one-to-many, many-to-one, or many-to-many references.

Example: 
The `tag` type provides an ad hoc many-to-many reference that can link together widely
disperate higher order structures.

Fields:
Uuid: A unique identifier for the Element Component.
Element_type: A name for the Element Component.

The Name field is a human readable name for the Element Component. It is trimmed of
end space and all lower case. It should not contain any special characters.
*/