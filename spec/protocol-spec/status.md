# Status Messages

Messages relaying different statuses, including communication
statuses, connection (ping), log messages, etc...

---
## Ok

**Description:** Signifies that the previous message sent by the
client was received and processed successfully by the server.

**Introduced In Spec Version:** 0

**Last Updated In Spec Version:** 0

**Fields:**

* _Id_ (unsigned int): The Id of the client message that this reply is
  in response to.

**Expected Response:**

None. Server-to-Client message only.

**Flow Diagram:**

<mermaid>
sequenceDiagram
    Client->>+Server: StartScanning Id=1
    Server->>-Client: Ok Id=1
</mermaid>

**Serialization Example:**

```json
[
  {
    "Ok": {
      "Id": 1
    }
  }
]
```
---
## Error

**Description:** Signifies that the previous message sent by the
client caused some sort of parsing or processing error on the server.

**Introduced In Spec Version:** 0

**Last Updated In Spec Version:** 0

**Fields:**

* _Id_ (unsigned int): The Id of the client message that this reply is
  in response to, assuming the Id could be parsed. Id will be 0 if
  message could not be parsed (due to issues like invalid JSON).
* _ErrorMessage_ (string): Message describing the error that
    happened on the server.
* _ErrorCode_ (int): Integer describing the error. Can be used in programs to react accordingly.
  * 0: ERROR\_UNKNOWN - An unknown error occurred.
  * 1: ERROR\_INIT - Handshake did not succeed.
  * 2: ERROR\_PING - A ping was not sent in the expected time.
  * 3: ERROR\_MSG - A message parsing or permission error occurred.
  * 4: ERROR\_DEVICE - A command sent to a device returned an error.

**Expected Response:**

None. Server-to-Client message only.

**Flow Diagram:**

<mermaid>
sequenceDiagram
    Client->>+Server: InvalidMsgName Id=2
    Server->>-Client: Error Id=2
</mermaid>

<mermaid>
sequenceDiagram
    Client->>+Server: InvalidMsgId Id=Wat
    Server->>-Client: Error Id=0
</mermaid>

**Serialization Example:**

```json
[
  {
    "Error": {
      "Id": 0,
      "ErrorMessage": "Server received invalid JSON.",
      "ErrorCode": 3
    }
  }
]
```
---
## Ping

**Description:** Ping acts a watchdog between the client and the
server. The server will expect the client to send a ping message at a
certain interval (interval will be sent to the client as part of the
identification step). If the client fails to ping within the specified
time, the server will disconnect and stop all currently connected
devices.

This will handle cases like the client crashing without a proper
disconnect. This is not a guaranteed global failsafe, since it will
not guard against problems like a client UI thread locking up while a
client communication thread continues to work.

**Introduced In Spec Version:** 0

**Last Updated In Spec Version:** 0

**Fields:**

* _Id_ (unsigned int): Message Id

**Expected Response:**

* Ok message with matching Id on successful ping.
* Error message on value or message error.

**Flow Diagram:**

<mermaid>
sequenceDiagram
    Client->>+Server: Ping Id=5
    Server->>-Client: Ok Id=5
</mermaid>

**Serialization Example:**

```json
[
  {
    "Ping": {
      "Id": 5
    }
  }
]
```
