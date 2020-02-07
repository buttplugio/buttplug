# How We Async

While our usage of async probably isn't anything all that new or special in the world of async Rust, we still get a lot of questions about how this design works. This page will hopefully answer some of those questions, or at least make people yell at us that we're doing it wrong.

## Why Async?

Async is fairly new in Rust, having only landed in the stable version of the language in November 2019. On top of that, it's mostly built with Rust's network-services heavy applications in mind. Async is quite handy when you need to deal with, say, 100000 http requests at a time through a system of other constrained services.

Buttplug, on the other hand, is a sex toy control library. This means accessing hardware, which in our case (using peripherals over USB, Bluetooth, Serial, etc...), this is almost always a slow, IO bound situation. Most people will only be using one sex toy at a time, possibly two or three in some instances, rarely more than 4-5.

So if there's not that much to orchestrate, why use async over threads? Rust makes threading a breeze, after all.

Buttplug's use of async is mostly from the following circumstances.

### Preference

Snapping back to first person for a sec because this answer is all about me.

I (qDot, the project lead) think I deal better mentally with async parallelism models than I do with threads. I've written a lot of both, and async just works better for me. As Buttplug is first and foremost my art project (its usage as a sex toy control library is a distance second), my feelings are top priority here. :3

### History

If you haven't yet, read the [history portion of the Why We Rust Section](why-we-rust.md). Notice how all of the implementations (outside of the original Rust impl) were async? This is following how we've done things so far.

## Why async-std and not Tokio?

buttplug-rs uses async Rust, with [async-std](https://github.com/async-rs/async-std) as our runtime and async library of choice.

Looking at the two libraries, async-std just made more sense to me up front. Async-std uses a Task system that's somewhat similar to C#'s, and while that's mostly a syntax deal, we'll take whatever we can get when we're trying to get a project off the ground.

This isn't a slight to tokio, which seems to be a very capable library that a lot of people get along quite well with, and honestly, if you're working outside Buttplug, there hopefully won't be too many conflicts on whatever runtime you use, since we try to just return Futures where we can.

## Event Loops, Threads, and Shims

All previously shipping implementations of Buttplug were in languages that had built-in event loops and runtimes. We now kinda get this with Rust async executors, but only kinda.

Not only that, Rust async is really, really new. This means a lot of the libraries we need to use to access hardware may not be async yet, so there's a chance they'll block, and therefore need their own thread even though the rest of our library is async.

This means that we have to spin up our own long-running tasks, or in some cases, full threads, to manage certain things. For instance, with a server, each device communication manager (things that handle a type of communication, like usb, bluetooth, etc) will get their own task/thread, because they'll need to constantly be keeping up with device events.

These structures will usually be called out in comments in the code.

Most information synchronization between tasks/threads/etc is handled using [async-std channels](https://docs.rs/async-std/1.5.0/async_std/sync/fn.channel.html). These are similar to [crossbeam's](https://github.com/crossbeam-rs/crossbeam) mpmc channels, though our channels will always be bounded so we just lock up versus running out of memory on error.

## Events

All previously shipping implementations of Buttplug were in languages that had first class events. Rust has no such facilities, so we've had to create our own.

As hardware can disconnect, or send data unprovoked, or do other things we don't request it to, events need to exist. To handle this, we create async _wait_for_event_ functions on our clients, devices, and servers. These wrap our internal channel implementations (see prior section for more info on that), and block on receive. The idea is that users of the library can create a task that, say, holds a device, and races 2 futures, one that waits for a message from outside to the command the device, and one that calls the devices _wait_for_event_ function. 

## Future Queuing

_TODO: Talk about how we shuttle futures around inside the library_
