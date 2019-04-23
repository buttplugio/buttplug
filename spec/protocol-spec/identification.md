# Handshake Messages

Messages used in the client/server handshake procedure.

---
## RequestServerInfo

**Description:** Sent by the client to register itself with the
server, and request info from the server.

**Introduced In Spec Version:** 0

**Last Updated In Spec Version:** 1 (See [Deprecated
Messages](deprecated.md#requestserverinfo-version-0) for older
versions.)

**Fields:**

* _Id_ \(unsigned int\): Message Id
* _ClientName_ \(string\): Name of the client, for the server to use
  for UI if needed. Cannot be null.
* _MessageVersion_ \(uint\): Message template version of the client
  software.

**Expected Response:**

* ServerInfo message on success
* Error message on malformed message, null client name, or other error.

**Flow Diagram:**

<mermaid>
sequenceDiagram
    Client->>Server: RequestServerInfo Id=0
    Server->>Client: ServerInfo Id=0
</mermaid>

**Serialization Example:**

```json
[
  {
    "RequestServerInfo": {
      "Id": 1,
      "ClientName": "Test Client",
      "MessageVersion": 1
    }
  }
]
```
---
## ServerInfo

**Description:** Send by server to client, contains information about
the server name \(optional\), template version, and ping time
expectations.

**Introduced In Spec Version:** 0

**Last Updated In Spec Version:** 2

**Fields:**

* _Id_ \(unsigned int\): Message Id
* _ServerName_ \(string\): Name of the server. Can be null \(0-length\).
* _MessageVersion_ \(uint\): Message template version of the server software.
* _MaxPingTime_ \(uint\): Maximum internal for pings from the client,
  in milliseconds. If a client takes to longer than this time between
  sending Ping messages, the server is expected to disconnect.

**Expected Response:**

None. Server-To-Client message only.

**Flow Diagram:**

<mermaid>
sequenceDiagram
    Client->>Server: RequestServerInfo Id=0
    Server->>Client: ServerInfo Id=0
</mermaid>

**Serialization Example:**

```json
[
  {
    "ServerInfo": {
      "Id": 1,
      "ServerName": "Test Server",
      "MessageVersion": 1,
      "MaxPingTime": 100
    }
  }
]
```

