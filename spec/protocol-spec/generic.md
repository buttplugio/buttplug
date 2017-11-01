# Generic Device Messages

## StopDeviceCmd

**Description:** Client request to have the server stop a device from  
whatever actions it may be taking. This message should be supported by  
all devices, and the server should know how to stop any device it  
supports.

**Message Version:** 0

**Fields:**

* _Id_ \(unsigned int\): Message Id
* _DeviceIndex_ \(unsigned int\): Index of device to stop.

**Expected Response:**

* Ok message with matching Id on successful request.
* Error message on value or message error.

**Flow Diagram:**

![img](stopdevice_diagram.svg)

**Serialization Example:**

```json
[
  {
    "StopDeviceCmd": {
      "Id": 1,
      "DeviceIndex": 0
    }
  }
]
```

## StopAllDevices

**Description:** Sent by the client to tell the server to stop all  
devices. Can be used for emergency situations, on client shutdown for  
cleanup, etc… While this is considered a Device Message, since  
it pertains to all currently connected devices, it does not specify a  
device index \(and does not end with 'Cmd'\).

**Message Version:** 0

**Fields:**

* _Id_ \(unsigned int\): Message Id

**Expected Response:**

* Ok message with matching Id on successful request.
* Error message on value or message error.

**Flow Diagram:**

![img](stopalldevices_diagram.svg)

**Serialization Example:**

```json
[
  {
    "StopAllDevices": {
      "Id": 1
    }
  }
]
```

## RawCmd

**Description:** Used to send a raw byte string to a device. Should  
only be used for development, and should not be exposed to untrusted  
clients.

**Message Version:** 0

**Fields:**

* _Id_ \(unsigned int\): Message Id
* _DeviceIndex_ \(unsigned int\): Index of device
* _Command_ \(Array of bytes\): Command to send, array of ints with a
  range of \[0-255\]. Minimum length is 1.

**Expected Response:**

* Ok message with matching Id on successful request.
* Error message on value or message error.

**Flow Diagram:**

![img](rawcmd_diagram.svg)

**Serialization Example:**

```json
[
  {
    "RawCmd": {
      "Id": 1,
      "DeviceIndex": 0,
      "Command": [0, 2, 4]
    }
  }
]
```

## SingleMotorVibrateCmd

**Description:** Causes a toy that supports vibration to run at a  
certain speed. In order to abstract the dynamic range of different  
toys, the value sent is a float with a range of \[0.0-1.0\]

**Message Version:** 0

**Fields:**

* _Id_ \(unsigned int\): Message Id
* _DeviceIndex_ \(unsigned int\): Index of device
* _Speed_ \(float\): Vibration speed

**Expected Response:**

* Ok message with matching Id on successful request.
* Error message on value or message error.

**Flow Diagram:**

![img](singlemotorvibratecmd_diagram.svg)

**Serialization Example:**

```json
[
  {
    "SingleMotorVibrateCmd": {
      "Id": 1,
      "DeviceIndex": 0,
      "Speed": 0.5
    }
  }
]
```



