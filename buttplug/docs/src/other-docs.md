# Things To Check Out Before You Start

There are a couple of other external documents that will be useful to
you as you read the Rust Buttplug Book.

- [The Buttplug Protocol Spec](https://buttplug-spec.docs.buttplug.io)
  defines the flow of the protocol that buttplug-rs implements. Many
  of the decisions in the library are based on the rules set out in
  this document, so it's good to have at least scanned it. Concentrate
  on the Intro, Architecture, and Messages portions. The actual
  message definitions themselves aren't as important as they're all
  implemented here.
- [STPIHKAL](https://stpihkal.docs.buttplug.io), aka Sex Toy Protocols
  I Have Known And Loved. This is where we document proprietary
  protocols that we implement here. We're perpetually behind in
  updating this, so it's also good to check the [github repo
  issues](https://github.com/buttplugio/stpihkal) to see what all is
  waiting to be documented.
- [The buttplug-rs API Docs](https://docs.rs/crate/buttplug/) are
  obviously going to be handy. However, the set on docs.rs only handle
  the publicly exposed API. Many private functions are commented in
  rustdoc format for local documentation usage, so if you plan on
  diving into the library code itself, it may be worth generating your
  own version of the documentation with all visibility levels turned
  on.
- The code itself is commented heavily, and is hopefully kind of
  readable. If nothing else, it's probably funny.
