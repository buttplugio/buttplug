# Status Messages

Messages relaying different statuses, including communication statuses, connection (ping), log messages, etc...

---
## Ok

**Description:** Signifies that the previous message sent by the client was received and processed successfully by the server.

**Introduced In Version:** 0

**Message Version:** 0

**Fields:**

* _Id_ \(unsigned int\): The Id of the client message that this reply is in response to.

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

**Description:** Signifies that the previous message sent by the client caused some sort of parsing or processing error on the server.

**Introduced In Version:** 0

**Message Version:** 0

**Fields:**

* _Id_ \(unsigned int\): The Id of the client message that this reply is in response to, assuming the Id could be parsed. Id will be 0 if message could not be parsed \(due to issues like invalid JSON\).
* _ErrorMessage_ \(string\): Message describing the error that
    happened on the server.
* _ErrorCode_ \(int\): Integer describing the error. Can be used in programs to react accordingly.
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

**Description:** Ping acts a watchdog between the client and the server. The server will expect the client to send a ping message at a certain interval \(interval will be sent to the client as part of the identification step\). If the client fails to ping within the specified time, the server will disconnect and stop all currently connected devices.

This will handle cases like the client crashing without a proper disconnect. This is not a guaranteed global failsafe, since it will not guard against problems like a client UI thread locking up while a client communication thread continues to work.

**Introduced In Version:** 0

**Message Version:** 0

**Fields:**

* _Id_ \(unsigned int\): Message Id

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
---
## Test

**Description:** The Test message is used for development and testing purposes. Sending a Test message with a string to the server will cause the server to return a Test message. If the string is "Error", the server will return an error message instead.

**Introduced In Version:** 0

**Message Version:** 0

**Fields:**

* _Id_ \(unsigned int\): Message Id
* _TestString_ \(string\): String to echo back from server.

**Expected Response:**

* Test message with matching Id and TestString on successful request.
* Error message on value or message error, or TestString being 'Error'.

**Flow Diagram:**

<mermaid>
sequenceDiagram
    Client->>+Server: Test Id=5 TestString=X
    Server->>-Client: Test Id=5 TestString=X
</mermaid>

**Serialization Example:**

```json
[
  {
    "Test": {
      "Id": 5,
      "TestString": "Moo"
    }
  }
]
```
---
## RequestLog

**Description:** Requests that the server send all internal log messages to the client. Useful for debugging.

**Introduced In Version:** 0

**Message Version:** 0

**Fields:**

* _Id_ \(unsigned int\): Message Id
* _LogLevel_ \(string\): The highest level of message to receive. Sending "Off" turns off messages, while sending "Trace" denotes that all log messages should be sent to the client. Valid LogLevel values:
  * Off
  * Fatal
  * Error
  * Warn
  * Info
  * Debug
  * Trace

**Expected Response:**

* Ok message with matching Id on successful logging request. Assuming the LogLevel was not "Off", Log type messages will be received after this.
* Error message on value or message error.

**Flow Diagram:**

<mermaid>
sequenceDiagram
    Client->>+Server: RequestLog Id=1
    Server->>-Client: Ok Id=1
</mermaid>

**Serialization Example:**

```json
[
  {
    "RequestLog": {
      "Id": 1,
      "LogLevel": "Warn"
    }
  }
]
```
---
## Log

**Description:** Log message from the server. Only sent after the client has sent a RequestLog message with a level other than "Off".

**Introduced In Version:** 0

**Message Version:** 0

**Fields:**

* _Id_ \(unsigned int\): Message Id
* _LogLevel_ \(string\): The level of the log message.
  * Off 
  * Fatal
  * Error
  * Warn
  * Info
  * Debug
  * Trace
* _LogMessage_ \(string\): Log message.

**Expected Response:**

None. Server-to-Client message only.

**Flow Diagram:**

<mermaid>
sequenceDiagram
    Client->>+Server: RequestLog Id=1
    Server->>-Client: Ok Id=1
    Server->>Client: Log Id=0 LogLevel=Warn
    Server->>Client: Log Id=0 LogLevel=Trace
</mermaid>

**Serialization Example:**

```json
[
  {
    "Log": {
      "Id": 0,
      "LogLevel": "Trace",
      "LogMessage": "This is a Log Message."
    }
  }
]
```
