# Spec Changelog

## Version 3 (2022-??-??)

- Messages Added:
  - ScalarCmd
    - Replaces VibrateCmd and adds ability to easily extend with new actuator types that take a
      single value.
  - SensorReadCmd
    - Replaces Battery/RSSI messages and adds ability to easily extend with new sensor types.
  - SensorSubscribeCmd
    - Allows users to receive realtime updates from devices (pressure sensors kegelcizers,
      accelerometers in toys that have them, etc...)
  - SensorUnsubscribeCmd
  - SensorReading
- Messages Changed:
  - DeviceList/DeviceAdded
    - Remove _FeatureCount_, Message Attributes are now an array of attribute objects instead of
      many fields of arrays that had to be reconstructed. Should reduce bookkeeping.
    - Added Message Attributes _FeatureDescriptor_, _ActuatorType_, _SensorType_
    - Added Device Attributes _DisplayName_
- Messages Deprecated:
  - VibrateCmd
    - Superceded by ScalarCmd. Will still be available via API calls in client APIs, just no longer
      needs to be a specific message in the protocol.
  - BatteryLevelCmd
    - Superceded by SensorReadCmd
  - RSSILevelCmd
    - Superceded by SensorReadCmd
  - BatteryLevelReading
    - Superceded by SensorReading
  - RSSILevelReading
    - Superceded by SensorReading

## Version 2 (2020-09-28)

- Messages Added:
  - RawWriteCmd
  - RawReadCmd
  - RawReading
  - RawSubscribeCmd
  - RawUnsubscribeCmd
  - BatteryLevelCmd
  - BatteryLevelReading
  - RSSILevelCmd
  - RSSILevelReading
- Messages Changed:
  - DeviceList/DeviceAdded
    - Adding StepCount to Message Attributes, to let users know how
      many steps a feature can use (i.e. how many vibration levels a
      piece of hardware might have)
  - ServerInfo
    - Remove Version fields
- Messages Deprecated:
  - LovenseCmd
    - Superceded by VibrateCmd/RotateCmd/Raw\*Cmd. The protocol messages were originally meant to
      map generic -> protocol -> raw, but the protocols change quickly enough that it's not worth it
      to encode that at the protocol level. From v2 of the spec on, we will try to encode as many
      actions as possible in generic messages. For anything we haven't mapped yet, Raw\*Cmd can be
      used, though it's not a great idea due to security concerns.
    - LovenseCmd was never implemented in any of the Buttplug reference libraries, so removal
      shouldn't affect anything.
  - KiirooCmd
    - Superceded by VibrateCmd/LinearCmd/Raw*Cmd. See above for more explanation.
    - Only implemented by the Kiiroo Pearl 1 and Onyx 1 in Buttplug C#. Not sure it was ever used
      anywhere.
  - VorzeA10CycloneCmd
    - Superceded by RotateCmd/PatternCmd. See above for more explanation.
    - Implemented for the Vorze A10 Cyclone in C# and JS, but translates directly to rotation
      messages.
  - FleshlightLaunchFW12Cmd
    - Superceded by LinearCmd/Raw\*Cmd. See LovenseCmd reason for more explanation.
    - Implemented for the Fleshlight Launch, and will be problematic to switch out. We should still
      support it on the server side for v0/v1 for compat.
  - Test
    - Violates assumptions that client/server sends different message types. Also, not particularly
      useful.
  - RequestLog/Log
    - Allows too much information leakage across the protocol in situations we may not want, and
      also has nothing to do with sex toy control. Logging is an application level function, not
      really required in the protocol itself.

## Version 1 (2017-12-11)

- Messages Added:
  - VibrateCmd
  - LinearCmd
  - RotateCmd
- Messages Changed:
  - DeviceList/DeviceAdded
    - Added Message Attributes blocks to device info, with FeatureCount attribute
  - RequestServerInfo
    - Added Spec Version Field
- Messages Deprecated:
  - SingleMotorVibrateCmd
    - Superceded by VibrateCmd

## Version 0 (2017-08-24)

- First version of spec
- Messages Added:
  - Ok
  - Error
  - Log
  - RequestLog
  - Ping
  - Test
  - RequestServerInfo
  - ServerInfo
  - RequestDeviceList
  - DeviceList
  - DeviceAdded
  - DeviceRemoved
  - StartScanning
  - StopScanning
  - ScanningFinished
  - SingleMotorVibrateCmd
  - FleshlightLaunchFW12Cmd
  - LovenseCmd
  - KiirooCmd
  - VorzeA10CycloneCmd
  - StopDeviceCmd
  - StopAllDevices
