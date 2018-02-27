# Identification Messages

## RequestServerInfo

**Description:** Sent by the client to register itself with the server, and request info from the server.

**Introduced In Version:** 0

**Message Version:** 1

**Fields:**

* _Id_ \(unsigned int\): Message Id
* _ClientName_ \(string\): Name of the client, for the server to use for UI if needed. Cannot be null.
* _MessageVersion_ \(uint\): Message template version of the client software.

**Expected Response:**

* ServerInfo message on success
* Error message on malformed message, null client name, or other error.

**Flow Diagram:**

![](blob:file:///ea7eba9e-a470-4494-89d3-20b8544da159)

**Serialization Example:**

```json
[
  {
    "RequestServerInfo": {
      "Id": 1,
      "ClientName": "Test Client",
      "MessageVersion": 1,
    }
  }
]
```

**Message Version:** 0

**Fields:**

* _Id_ \(unsigned int\): Message Id
* _ClientName_ \(string\): Name of the client, for the server to use for UI if needed. Cannot be null.

**Expected Response:**

* ServerInfo message on success
* Error message on malformed message, null client name, or other error.

**Flow Diagram:**

![](blob:file:///ea7eba9e-a470-4494-89d3-20b8544da159)

**Serialization Example:**

```json
[
  {
    "RequestServerInfo": {
      "Id": 1,
      "ClientName": "Test Client"
    }
  }
]
```

## ServerInfo

**Description:** Send by server to client, contains information about the server name \(optional\), template version, and ping time expectations.

**Introduced In Version:** 0

**Message Version:** 0

**Fields:**

* _Id_ \(unsigned int\): Message Id
* _ServerName_ \(string\): Name of the server. Can be null \(0-length\).
* _MajorVersion_ \(uint\): Major version of the server software.
* _MinorVersion_ \(uint\): Minor version of the server software.
* _BuildVersion_ \(uint\): Build version of the server software.
* _MessageVersion_ \(uint\): Message template version of the server software.
* _MaxPingTime_ \(uint\): Maximum internal for pings from the client, in milliseconds. If a client takes to longer than this time between sending Ping messages, the server is expected to disconnect.

**Expected Response:**

None. Server-To-Client message only.

**Flow Diagram:**

![img](serverinfo_diagram.svg)

**Serialization Example:**

```json
[
  {
    "ServerInfo": {
      "Id": 1,
      "ServerName": "Test Server",
      "MessageVersion": 1,
      "MajorVersion": 1,
      "MinorVersion": 0,
      "BuildVersion": 0,
      "MaxPingTime": 100
    }
  }
]
```



