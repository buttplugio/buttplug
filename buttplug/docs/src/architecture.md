# Architecture

**Note:** This is just cut and pasted from what I originally wrote in the README. It definitely needs updating.

This section is a discussion of the idea behind the new (Sept 2019) Rust implementation of Buttplug.

Understanding the following discussion is probably only possible if you check out the [Buttplug Protocol Spec](https://buttplug-spec.docs.buttplug.io) first. This also assumes some knowledge of Buttplug reference library architecture. If you're really curious and have questions, [join our discord and ask qDot](https://discord.buttplug.io).

The Rust implementation of Buttplug is meant to be a canonical, reusable core library, accessible from other programming languages via FFI boundaries.

The core library consists of:

- Core
  - Protocol Message definitions, mostly.
- Devices
  - Proprietary device protocols. These take in Buttplug Control Commands, output ButtplugRawCmds. Similarly, they can also translate and emit input messages from hardware.
  - Device configuration file handling. This is the external file we use for defining device identifiers. This includes USB Vendor/Product IDs, Bluetooth LE Names and Service/Characteristic UUIDs, etc.
- Server
  - Coordinates talking to hardware, via DeviceSubtypeManager objects. DeviceSubtypeManagers handle platform specific communication bus access, i.e. USB, Bluetooth, Serial, etc. **NOTE**: DeviceSubtypeManagers may exist across an FFI boundry. They are not required to be implemented in Rust.
- Client
  - API used by applications to communicate with a server. Abstracts communication via connectors, so that a Server can exist in-process, or out-of-process via some sort of IPC (pipes, websockets, carrier pidgeon, etc...). Also exposes FFI bindings for client access in languages other than Rust.

Things not in the core library but still needed by Buttplug include
either platform/OS specific or optional components, such as:

- DeviceSubtypeManagers
  - Talking to Bluetooth/USB/Serial/Etc usually takes platform specific calls, which sometimes may be easier to make from other languages. For instance, calling UWP Bluetooth on Windows is easier from C#. Therefore, we maintain an FFI boundary with a protobuf protocol here.
- Client Connectors
  - Clients may need to talk to Servers that exist in other processes. Therefore we provide Connectors, which allow different types of IPC. These feed clients with messages from the IPC, and are usually external implementations as not all uses of Buttplug will require all types of IPC.

In prior implementations of Buttplug, including C# and Typescript, the core usually existed expecting implementations of DeviceSubtypeManagers and Connectors in the same language. As both of these languages require specific runtimes (.Net for C#, node or a browser for Typescript/Javascript), making them cross platform required rewriting the core implementation. By using Rust, the hope is to keep one reference implementation of the core library, and wrapping that in FFIs for other implementations or compiling to WASM or other intermediaries as needed.

This is hopefully easier than, say, hauling the whole .Net platform along with us everywhere we go.

To support other languages, the plan is to have multiple FFI boundaries and possible IPC tunnels.

- The client API will be accessible via FFI, so we can build language bindings on top of it.
- The layer between the Client and Server has always existed as an IPC boundary outside of cases where the client/server are in the same process.
- The server will have an FFI boundary on devices, so we can implement platform specific device handling in other languages if needed (Swift, Java, C#, etc.)

Incoming messages will still be JSON, because we made a bad decision early on and by god we're sticking with it. Internal boundaries (like the server/DeviceSubtypeManager boundary) will either be callback based, or message-passing via protobuf protocols.
