# Specific Device Messages


## KiirooCmd

**Description:** Causes a toy that supports Kiiroo style commands to
run whatever event may be related. More information on Kiiroo commands
can be found in STPIHKAL.

**Fields:**

- *Id* (unsigned int): Message Id
- *DeviceIndex* (unsigned int): Index of device
- *Command* (unsigned int): Unsigned integer in range [0-4].

**Expected Response:**

- Ok message with matching Id on successful request.
- Error message on value or message error.

**Flow Diagram:**

![img](kiiroocmd_diagram.svg)

**Serialization Example:**

```json
[
  {
    "KiirooCmd": {
      "Id": 1,
      "DeviceIndex": 0,
      "Command": 4
    }
  }
]
```


## FleshlightLaunchFW12Cmd

**Description:** Causes a toy that supports Fleshlight Launch
(Firmware Version 1.2) style commands to run whatever event may be
related. More information on Fleshlight Launch commands can be found
in STPIHKAL.

**Fields:**

- *Id* (unsigned int): Message Id
- *DeviceIndex* (unsigned int): Index of device
- *Position* (unsigned int): Unsigned integer in range [0-99],
    denoting position to move to.
- *Speed* (unsigned int): Unsigned integer in range [0-99], denoting
    speed to requested position at.

**Expected Response:**

- Ok message with matching Id on successful request.
- Error message on value or message error.

**Flow Diagram:**

![img](fleshlightlaunchfw12cmd_diagram.svg)

**Serialization Example:**

```json
[
  {
    "FleshlightLaunchFW12Cmd": {
      "Id": 1,
      "DeviceIndex": 0,
      "Position": 95,
      "Speed": 90
    }
  }
]
```


## LovenseCmd

**Description:** Causes a toy that supports Lovense style commands to
run whatever event may be related. More information on Lovense
commands can be found in STPIHKAL.

**Fields:**

- *Id* (unsigned int): Message Id
- *DeviceIndex* (unsigned int): Index of device
- *Command* (string): String command for Lovense toys. Must be a
    valid Lovense command accessible on most of their toys. See
    STPIHKAL for more info. Implementations should check this for
    validity.

**Expected Response:**

- Ok message with matching Id on successful request.
- Error message on value or message error.

**Flow Diagram:**

![img](lovensecmd_diagram.svg)

**Serialization Example:**

```json
[
  {
    "LovenseCmd": {
      "Id": 1,
      "DeviceIndex": 0,
      "Command": "Vibrate:20;"
    }
  }
]
```
