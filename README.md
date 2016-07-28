# csharp_deserialize
C# binary files deserialize to json, written in Rust.

Based on discussions on https://stackoverflow.com/questions/3052202/how-to-analyse-contents-of-binary-serialization-stream, and of course the MS official documents.

Not all data types are addressed yet. Basically this is for extracting infomations from Unity games.

##Usage
Drag&drop C# serialized files to the executable and then json files will be created. Only files with *bytes* extension will be addressed. 
