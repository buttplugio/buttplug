# Device Definitions

This document describes the three canonical test devices used in the conformance test harness. These devices are simulated by the harness and reported via the DeviceList message after scanning.

---

## Device 0: Conformance Test Vibrator

**Device Index:** 0  
**Device Name:** Conformance Test Vibrator

### Feature Table

| Feature Index | Description | Output Type | Output Range | Input Type | Input Range | Commands |
|--------------|-------------|------------|-------------|-----------|------------|----------|
| 0 | Vibrator 1 | Vibrate | 0–100 | — | — | OutputCmd |
| 1 | Vibrator 2 | Vibrate | 0–100 | — | — | OutputCmd |
| 2 | Rotator | Rotate | -100–100 | — | — | OutputCmd |
| 3 | Battery | — | — | Battery | 0–100 | InputCmd (Read) |

### Example Commands

**Vibrate Feature 0:**
```json
[{"OutputCmd": {"Id": 1, "DeviceIndex": 0, "FeatureIndex": 0, "Command": {"Vibrate": {"Value": 50}}}}]
```

**Vibrate Feature 1:**
```json
[{"OutputCmd": {"Id": 2, "DeviceIndex": 0, "FeatureIndex": 1, "Command": {"Vibrate": {"Value": 75}}}}]
```

**Rotate Feature 2:**
```json
[{"OutputCmd": {"Id": 3, "DeviceIndex": 0, "FeatureIndex": 2, "Command": {"Rotate": {"Value": -50}}}}]
```

**Read Battery (Feature 3):**
```json
[{"InputCmd": {"Id": 4, "DeviceIndex": 0, "FeatureIndex": 3, "Type": "Battery", "Command": "Read"}}]
```

Server will respond with:
```json
[{"InputReading": {"Id": 0, "DeviceIndex": 0, "FeatureIndex": 3, "Reading": {"Battery": {"Value": 85}}}}]
```

---

## Device 1: Conformance Test Positioner

**Device Index:** 1  
**Device Name:** Conformance Test Positioner

### Feature Table

| Feature Index | Description | Output Type | Output Range | Input Type | Input Range | Commands |
|--------------|-------------|------------|-------------|-----------|------------|----------|
| 0 | Position | Position | 0–100 | — | — | OutputCmd |
| 1 | Position w/ Duration | HwPositionWithDuration | 0–100, 0–10000ms | — | — | OutputCmd |
| 2 | Oscillator | Oscillate | 0–100 | — | — | OutputCmd |
| 3 | Button | — | — | Button | 0–1 | InputCmd (Subscribe/Unsubscribe) |

### Example Commands

**Position Feature 0:**
```json
[{"OutputCmd": {"Id": 5, "DeviceIndex": 1, "FeatureIndex": 0, "Command": {"Position": {"Value": 50}}}}]
```

**HwPositionWithDuration Feature 1:**
```json
[{"OutputCmd": {"Id": 6, "DeviceIndex": 1, "FeatureIndex": 1, "Command": {"HwPositionWithDuration": {"Value": 75, "Duration": 2000}}}}]
```

**Oscillate Feature 2:**
```json
[{"OutputCmd": {"Id": 7, "DeviceIndex": 1, "FeatureIndex": 2, "Command": {"Oscillate": {"Value": 60}}}}]
```

**Subscribe to Button (Feature 3):**
```json
[{"InputCmd": {"Id": 8, "DeviceIndex": 1, "FeatureIndex": 3, "Type": "Button", "Command": "Subscribe"}}]
```

Server will push button events:
```json
[{"InputReading": {"Id": 0, "DeviceIndex": 1, "FeatureIndex": 3, "Reading": {"Button": {"Value": 1}}}}]
```

**Unsubscribe from Button:**
```json
[{"InputCmd": {"Id": 9, "DeviceIndex": 1, "FeatureIndex": 3, "Type": "Button", "Command": "Unsubscribe"}}]
```

---

## Device 2: Conformance Test Multi

**Device Index:** 2  
**Device Name:** Conformance Test Multi

### Feature Table

| Feature Index | Description | Output Type | Output Range | Input Type | Input Range | Commands |
|--------------|-------------|------------|-------------|-----------|------------|----------|
| 0 | Constrictor | Constrict | 0–100 | — | — | OutputCmd |
| 1 | Sprayer | Spray | 0–100 | — | — | OutputCmd |
| 2 | Heater | Temperature | -100–100 | — | — | OutputCmd |
| 3 | LED | Led | 0–100 | — | — | OutputCmd |
| 4 | RSSI | — | — | Rssi | -128–0 | InputCmd (Read) |
| 5 | Pressure | — | — | Pressure | 0–65535 | InputCmd (Subscribe/Unsubscribe) |

### Example Commands

**Constrict Feature 0:**
```json
[{"OutputCmd": {"Id": 10, "DeviceIndex": 2, "FeatureIndex": 0, "Command": {"Constrict": {"Value": 50}}}}]
```

**Spray Feature 1:**
```json
[{"OutputCmd": {"Id": 11, "DeviceIndex": 2, "FeatureIndex": 1, "Command": {"Spray": {"Value": 75}}}}]
```

**Temperature Feature 2:**
```json
[{"OutputCmd": {"Id": 12, "DeviceIndex": 2, "FeatureIndex": 2, "Command": {"Temperature": {"Value": -20}}}}]
```

**Led Feature 3:**
```json
[{"OutputCmd": {"Id": 13, "DeviceIndex": 2, "FeatureIndex": 3, "Command": {"Led": {"Value": 100}}}}]
```

**Read RSSI (Feature 4):**
```json
[{"InputCmd": {"Id": 14, "DeviceIndex": 2, "FeatureIndex": 4, "Type": "Rssi", "Command": "Read"}}]
```

Server will respond with:
```json
[{"InputReading": {"Id": 0, "DeviceIndex": 2, "FeatureIndex": 4, "Reading": {"Rssi": {"Value": -50}}}}]
```

**Subscribe to Pressure (Feature 5):**
```json
[{"InputCmd": {"Id": 15, "DeviceIndex": 2, "FeatureIndex": 5, "Type": "Pressure", "Command": "Subscribe"}}]
```

Server will push pressure readings:
```json
[{"InputReading": {"Id": 0, "DeviceIndex": 2, "FeatureIndex": 5, "Reading": {"Pressure": {"Value": 32768}}}}]
```

**Unsubscribe from Pressure:**
```json
[{"InputCmd": {"Id": 16, "DeviceIndex": 2, "FeatureIndex": 5, "Type": "Pressure", "Command": "Unsubscribe"}}]
```

---

## DeviceList Response

After sending `StartScanning`, the client receives a complete DeviceList with all three devices. This is the canonical structure:

```json
[{"DeviceList": {"Id": 0, "Devices": {
  "0": {
    "DeviceIndex": 0,
    "DeviceName": "Conformance Test Vibrator",
    "DeviceMessageTimingGap": 0,
    "DeviceFeatures": {
      "0": {"FeatureIndex": 0, "FeatureDescription": "Vibrator 1", "Output": {"Vibrate": [[0, 100]]}},
      "1": {"FeatureIndex": 1, "FeatureDescription": "Vibrator 2", "Output": {"Vibrate": [[0, 100]]}},
      "2": {"FeatureIndex": 2, "FeatureDescription": "Rotator", "Output": {"Rotate": [[-100, 100]]}},
      "3": {"FeatureIndex": 3, "FeatureDescription": "Battery", "Input": {"Battery": {"Value": [[0, 100]], "Command": ["Read"]}}}
    }
  },
  "1": {
    "DeviceIndex": 1,
    "DeviceName": "Conformance Test Positioner",
    "DeviceMessageTimingGap": 0,
    "DeviceFeatures": {
      "0": {"FeatureIndex": 0, "FeatureDescription": "Position", "Output": {"Position": [[0, 100]]}},
      "1": {"FeatureIndex": 1, "FeatureDescription": "Position w/ Duration", "Output": {"HwPositionWithDuration": {"Value": [[0, 100]], "Duration": [[0, 10000]]}}},
      "2": {"FeatureIndex": 2, "FeatureDescription": "Oscillator", "Output": {"Oscillate": [[0, 100]]}},
      "3": {"FeatureIndex": 3, "FeatureDescription": "Button", "Input": {"Button": {"Value": [[0, 1]], "Command": ["Subscribe", "Unsubscribe"]}}}
    }
  },
  "2": {
    "DeviceIndex": 2,
    "DeviceName": "Conformance Test Multi",
    "DeviceMessageTimingGap": 0,
    "DeviceFeatures": {
      "0": {"FeatureIndex": 0, "FeatureDescription": "Constrictor", "Output": {"Constrict": [[0, 100]]}},
      "1": {"FeatureIndex": 1, "FeatureDescription": "Sprayer", "Output": {"Spray": [[0, 100]]}},
      "2": {"FeatureIndex": 2, "FeatureDescription": "Heater", "Output": {"Temperature": [[-100, 100]]}},
      "3": {"FeatureIndex": 3, "FeatureDescription": "LED", "Output": {"Led": [[0, 100]]}},
      "4": {"FeatureIndex": 4, "FeatureDescription": "RSSI", "Input": {"Rssi": {"Value": [[-128, 0]], "Command": ["Read"]}}},
      "5": {"FeatureIndex": 5, "FeatureDescription": "Pressure", "Input": {"Pressure": {"Value": [[0, 65535]], "Command": ["Subscribe", "Unsubscribe"]}}}
    }
  }
}}}]
```

---

## Notes

- All device indices are fixed: Device 0, 1, 2 throughout all sequences.
- Output commands wrap values in envelopes matching the feature type.
- Most output commands use `Value` (Vibrate, Rotate, Constrict, Spray, Temperature, Led, Position, Oscillate).
- HwPositionWithDuration uses both `Value` (0–100) and `Duration` (0–10000 ms).
- Input commands (InputCmd) require a `Type` field (Battery, Rssi, Button, Pressure) and can be "Read" (one-time) or "Subscribe"/"Unsubscribe" (streaming).
- InputReading responses carry sensor values in a `Reading` object with the sensor type as the key. Format varies by sensor type:
  - Battery: `{"Battery": {"Value": 0-100}}`
  - Button: `{"Button": {"Value": 0-1}}`
  - Rssi: `{"Rssi": {"Value": -128 to 0}}`
  - Pressure: `{"Pressure": {"Value": 0-65535}}`
