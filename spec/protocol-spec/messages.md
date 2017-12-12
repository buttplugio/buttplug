# Messages

Messages are the core of communication for Buttplug.

How messages are represented depends on the implementation in question. For instance, in a C\# library implementation of Buttplug, messages are classes. In Rust, they're structs.

In a server implementation, messages need to be serialized in some way to be sent between the client and server. In this case, they may exist in some sort of intermediate format, like JSON or OSC.

## Basic Message Structure

Messages are made up of multiple different kinds of fields. As long as the fields can somehow be represented in JSON, we consider them valid.

All messages will contain an "Id" field. This field as the range of 0 to 4294967295. A value to 0 denotes a _System_ message, meaning a message that will only ever be sent from a server to a client. All messages coming from a client will have an Id from 1 to 4294967295, as set by the client themselves. When the server replies to the message, it will return a message using the same Id as was sent. This allows developers to synchronize messages over remote systems like networks, or languages that lack async/await capabilities.

Other than range, there is no restriction to what values the client can send as an Id. The Id does not need to be sequential, nor does it need to be unique. The client could just send 1 for every message, which would be valid in async/await library situations where the execution flow would handle matching message pairs without the need for the Id. In remote situations, like those over network connections, it is expected that the client will establish a sane usage of the Id field to orchestrate messaging.

## Message Flow

There are two types of message flows.

* Messages can be sent from the server to a client. Messages like DeviceAdded, DeviceRemoved, and certain device specific input messages can happen without the client making a request. The server will not expect a reply from the client for these messages.
* Messages sent from the client to the server will always receive a reply from the server. The message type the client will receive in reply is based on the type of message sent. Some messages may receive a simple "Ok" message in reply in order to denote successful receiving, while others may receive something context specific. Messages reply types are listed in the message descriptions section.

## A Note On Scaling

The Buttplug Message System, as described here, was not designed to scale to large multiuser systems \(like cam services\). It was built with either a single user, peer-to-peer, or small group setting in mind.

As the message flow section states, this system resembles a sort of half-assed-TCP mechanism. Using this system to drive large scale toy control streaming services may require changes to this system.

Reducing and rearchitecting this system for scaling is an exercise left to the developer. Either to implement, or to contract the Buttplug designers to build it for them.

## JSON Message Serialization

For reference implementations of the Buttplug standard, we use JSON for serialization. The format of the json object for each specific message mimics that of object output from Rust's [serde-json](https://github.com/serde-rs/json) crate. This is simply due to the first implementation of Buttplug with working serialization being in Rust.

When sending messages over the line to a server/client, we wrap them in a JSON array, so that multiple messages can be sent and parsed simultaneously.

The format is as follows:

```json
[
  {
    "MessageType" :
    {
      "MessageField1": "MessageValue1",
      "MessageField2": "MessageValue2"
    }
  },
  {
    "MessageType2" :
    {
      "Message2Field1": "Message2Value1",
      "Message2Field2": "Message2Value2"
    }
  }
]
```

Message descriptions in this document will reflect this layout.

Similarly, some message values will have certain bounds and limitations. These are described in this documentation, and are included in the JSON schema in this repo.

## Adding New Messages

The message list as described here is not set in stone. New messages will be added as new devices are released, or as new generic messages are deemed necessary. The only rule is that once a message is added to this document, it should never be removed; however, newer versions of the message may suceed it. This will allow parsing andschema checking to be as strict as possible. If edits to a message need to be made, a new message type will most likely be added.

Requests for new messages can be submitted to [the Buttplug Standard Github Issue Tracker](https://github.com/metafetish/buttplug/issues).

## Message Versioning

In order to cope with changes to the schema across servers and clients that may not both support the same schema versions, each message type defined in this document has a message version number. These are based on the schema version they were introduced in, represented as an unsigned integer. As of version 1, this document and the schema are synchronized by using the message version over any other form of release number. Version 0 was represented as release 0.1.0.

The message versions are not contained in the messages themselves, instead the client sends the overall schema message version as part of the RequestServerInfo message and the server includes its schema message version in the ServerInfo response. Note that the original version of RequestServerInfo does not have a parameter for this, so the servers that implement version 0 will reject clients capable of using the version 1 schema. This is not seen as an issue, as the server is likely to be the first component in the Buttplug archtecture to be updated, and is the component most likely to be under the end-user's control to update.

In the case that the server supports a newer schema than the client, any messages that the server attempts to send will be checked against the message version and either downgraded to a previous version where possible, or simply dropped.
