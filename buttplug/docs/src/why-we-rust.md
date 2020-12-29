# Why We Rust

The Rust implementation of Buttplug will be the 4th implementation of both the client and server portions of the library to date. As the client only requires implementing messages and some network/IPC communication, more implementations of the client exist, but the server is the where the complexity lies.

The history of Buttplug looks like this:

* 2013: _Python 2 + greenlet_
  * The first, unreleased implementation of buttplug. [It's even in our repos if you want to check it out](https://github.com/buttplugio/buttplug-py-deprecated). This used python 2, greenlet, and ZMQ, but never got as far as talking to a device. It was mostly me playing around with architecture. The project was abandoned because I couldn't figure out how I was going to distribute it easily.
* 2016: _Rust_
  * Yes, Buttplug was implemented in Rust once already. Sort of. Due to the lack of hardware libraries, windows support (WinAPI 0.3 wasn't out yet), etc, this version only lived for about a month before being abandoned.
* 2017: _C#_
  * The second, and most mature implementation of Buttplug. C# was chosen because of Windows compatibility (though all libraries are now .Net Standard and compile cross-platform), which is where most of our users are. This implementation is where the current version of the spec came from. It uses C#'s async/Task features, and the design of this version of the library heavily influenced buttplug-rs.
* 2017: _Javascript_
  * In an effort to build a pure web version of Buttplug, a Javascript (well, actually Typescript, but you get it) implementation was created. This ended up being both a pure web library (accessing devices through WebBluetooth), as well as a node library (accessing devices via noble). Due to the inherently async nature of Javascript engines, this was an async implementation, using promises and es7 async/await. This version has constantly lagged behind C# mostly because maintaining multiple libraries sucks.

The split between C# and Javascript also helped us support as many platforms as possible. Going into development on buttplug-rs, our platform supports looked like this:

* Windows - C# (Node compiles, but is slow and difficult)
* Mac - JS/Node (C# compiles, no Bluetooth/USB)
* Linux - JS/Node (C# compiles, no Bluetooth/USB)
* Android - Xamarin (C#, Bluetooth via Xamarin Bluetooth)
* iOS - Xamarin (C#, Bluetooth via Xamarin Bluetooth)
* Web - Pure JS (Blink Engines only for WebBluetooth, so users required Chrome or Edge to use in browser)

Needless to say, the fragmentation between the libraries was a problem. None of our users were sure when or how their devices would work. This, combined with the new fragmentation of C# 8.0/.Net Core 3, and the Xamarin lockin on mobile, meant we either needed to put all of our eggs in the .Net basket, or else look at another solution that could get us native everywhere.

Evaluating Rust in late 2019 was a far different situation than it was in 2016. FFI was more mature, WinAPI 0.3 was out and WinRT-rs provided UWP support, multiple Bluetooth libraries had already been written (though none were fully cross platform, [we fixed that](https://github.com/deviceplug/btleplug)), async/await was on the way, many projects were compiling Rust native to mobile platforms and using Java or Swift via FFI on top of it, and compiling to WASM is an option (albeit still a difficult one). Choosing Rust now would at least get us close to parity with where we were, all in the same language, and we'll be able to progress with the community and technology as it grows.

Of course, coming from GC'd, Runtime'd languages, the jump to the native compiled world of Rust isn't going to be trivial by any means, but how we've dealt with the change is documented as part of this book.

Anyways, that's why this book is being written. The new (and hopefully forever) plan is to keep a single core reference implementation of Buttplug in Rust, then FFI to other languages (including C#) on top of that, so the most we implement across multiple languages are the FFI shims.

May god have mercy on us all.
