# Spec Changelog

## Version 2 (In Development)

- Messages Added:
  - RawWriteCmd
  - RawReadCmd
  - RawReading
  - RawSubscribeCmd
  - RawUnsubscribeCmd
  - ShockCmd
  - PatternCmd
  - BatteryLevelCmd
  - BatteryLevelReading
  - RSSILevelCmd
  - RSSILevelReading
- Messages Changed:
  - DeviceList/DeviceAdded
    - Adding StepCount to Message Attributes, to let users know how
      many steps a feature can use (i.e. how many vibration levels a
      piece of hardware might have)
    - Adding PatternNames mapping to Message Attributes for anything that
      supports PatternCmd.
  - ServerInfo
    - Remove Version fields
- Messages Deprecated:
  - LovenseCmd
    - Superceded by VibrateCmd/RotateCmd/Raw*Cmd. The protocol
      messages were originally meant to map generic -> protocol ->
      raw, but the protocols change quickly enough that it's not worth
      it to encode that at the protocol level. From v2 of the spec on,
      we will try to encode as many actions as possible in generic
      messages. For anything we haven't mapped yet, Raw*Cmd can be
      used, though it's not a great idea due to security concerns.
  - KiirooCmd
    - Superceded by VibrateCmd/LinearCmd/Raw*Cmd. See above for more
      explanation.
  - VorzeA10CycloneCmd
    - Superceded by RotateCmd/PatternCmd. See above for more
      explanation.
  - FleshlightLaunchFW12Cmd
    - Superceded by LinearCmd/Raw*Cmd. See above for more explanation.

## Version 1 (2017-12-11)

- Messages Added:
  - VibrateCmd
  - LinearCmd
  - RotateCmd
- Messages Changed:
  - DeviceList/DeviceAdded
    - Added Message Attributes blocks to device info.
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
