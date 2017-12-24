# The Buttplug Intimate Device Control Standard

* **Version:** 1
* **Documentation Repo**: [https://github.com/metafetish/buttplug](https://github.com/metafetish/buttplug)

Buttplug is a quasi-standard set of technologies and protocols to allow developers to write software that can access an array of computer controlled devices (sex toys, estim hardware, kegelcizers, etc...) in a semi-future-proof way.

## The Need for a Computer Controlled Intimate Device Protocol Standard

If a user wants to access a computer controlled sex device in a way not supported by the original manufacturer, the requirement list to is not trivial. There are some major hurdles between obtaining a device and having DIY control of it:

* Experience in the operating system the user will want to access the device from, including capabilities and programming interfaces to work with the connection medium \(serial/usb/bluetooth/etc\)
* Experience using a programming language that allows the user access to hardware via the operating system.
* Knowledge of the communications protocol the device uses. This is rarely, if ever, publicly documented information.

The Buttplug Sex Device Control Standard seeks to lower these bars as much as possible.

* By proposing a standard that can be implemented in a cross-platform way, software based on the Buttplug Standard can reduce the amount of knowledge required to access hardware from a certain operating system or platform. Using technologies like Cordova or Xamarin to build software that implements Buttplug means that access could happen via desktop or mobile platforms.
* By standardizing the methods that can be used to talk to these devices, implementations of the standard can happen in multiple languages and still interact with each other. This opens development opportunities to multiple communities and ecosystems.
* Assuming some sort of widespread adoption happens, this could drive the commercial market to build devices with the Buttplug Standard in mind, or even to use it directly. Until that point, the portion of the community familiar with reverse engineering can help open device access to those who are interested in controlling the devices.

## Generalized Control

One of the windmills Buttplug tilts at is the idea of "generalized control". Simply put, is it viable to drive completely different devices from the same control signal? In some cases, this is simple. In others, it may be impossible.

Starting with the simple case, let's say a user has two devices, called Device A and Device B:

* Each device is made by a different manufacturer.
* One device uses Bluetooth to talk to the computer, the other uses USB.
* Both of these devices have vibration functionality.

The user has a particular function they would like to implement in a software application, which would utilize the vibration function of both devices.

Without Buttplug, this would require knowing how to talk to both USB and Bluetooth, and also knowing how each of these devices communicates with the computer in order to control vibration levels. They would then have to add both of these to their application.

With Buttplug, Server implementations are expected to take care of the different manufacturer and hardware communication requirements. However, if there were only ways to communicate with specific hardware, the application they were making would have to provide separate logic paths to cover either device instance.

This is where the idea of "generalized haptics" comes in. Instead of either a "Device A" or "Device B" command to the server, the user can just send a "VibrateCmd" command to the server, along with the identifier for which device they wanted to use. Not only that, their software would work with any device \(including devices they do not own/have tested with\) that could translate the "VibrateCmd" command.

Now, the not so simple case.

Let's add Device C, an electrostimulation unit. To use Device C with the same application as Device A and Device B, the Vibrate command has to be translated into some facsimile that is valid for estim. While this is most likely not tractable for a global solution, the goal of Buttplug is to make explorations of ideas like this accessible and easy to play with.

## Comparisons to Existing Software

It's somewhat difficult to point to a real world counterpart for the Buttplug Standard. While companies like [Frixion](http://twitter.com/frixionme) and [FeelMe](http://feelme.com) have created systems for controlling different devices, neither of those is open source, so it's hard to point at them as examples.

The closest existing projects are those which reinterpret or generalize control schemes. Projects like:

* [FreePIE](http://andersmalmgren.github.io/FreePIE/)
* [OSCulator](https://osculator.net/)
* [VRPN](https://github.com/vrpn/vrpn/wiki)
* [vJoy](http://vjoystick.sourceforge.net/site/)

All of these programs take input from various devices and translate them as some other kind of input, or aggregate them to make systems easier to use. The Buttplug Standard aims to define programs which do something similar. Applications referred to as "Buttplug Server" implementations will often look quite similar to these programs.

## Why is it called Buttplug?

It probably seems silly to call a generic device control standard "Buttplug".

That's because it is.

I could probably call this project something neutral like Sex Device Control Standard (SDCS?), but I've been referring to computer controlled sex devices as "Internet Buttplugs" for years, and that's what influenced the name of this project. It's hard to pick terms for these products.

* "Sex toy" is weighed down by the word "toy". This is part of the reason the academic and tech community is flocking toward "sex robot" even when discussing technology that would've been called a sex toy a decade ago.
* "Sex robot" has way too many connotations, be it Cherry 2000 or robotics academics writing media-friendly PhD theses.
* "Sex device" is used in this document, but feels awkward for reasons I'm still figuring out.
* "Adult novelty" just sounds stale and corporate. You buy adult novelties in bulk from warehouses. You go to adult novelty conventions.
* "Marital aide" No.

I ended up with "Internet Buttplug" because everyone has a butt, and buttplug is a fun word to say. It's inclusive and it's humorous. I admit that it may confuse people when they're wondering why they're using something called Buttplug to control their fucking machine or robotic onahole or who knows what else.

One of the hardest problems in Computer Science is naming things. I just stopped trying to name the thing and selected a name and here we are. Much like the other hard problems in Computer Science, I fully expect this to come back to bite me in the ass at some point.

**Please Note:** Even though this project is called Buttplug, it does not mean you have to put something in your butt to develop with it or use applications that integrate it. We are not saying you shouldn't, as we condone butt stuff as performed in a safe and sane manner, but it's not a requirement, either.

