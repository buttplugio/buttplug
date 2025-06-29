const yaml = require('js-yaml');
const uuid = require('uuid');
const fs = require('fs');
// Get document, or throw exception on error
const doc = yaml.load(fs.readFileSync('./device-config-v3/buttplug-device-config-v3.yml', 'utf8'));
for (var protocol in doc["protocols"]) {
  console.log(protocol);
  if (doc["protocols"][protocol]["defaults"] !== undefined) {
    if (doc["protocols"][protocol]["defaults"]["id"] === undefined) {
      doc["protocols"][protocol]["defaults"]["id"] = uuid.v4();
    }
    for (var feature of doc["protocols"][protocol]["defaults"]["features"]) {
      if (feature["id"] === undefined) {
        feature["id"] = uuid.v4();
      }
    }
  }
  if (doc["protocols"][protocol]["configurations"] !== undefined) {
    for (var config of doc["protocols"][protocol]["configurations"]) {
      if (config["id"] === undefined) {
        config["id"] = uuid.v4();
      }
      if (config["features"] !== undefined) {
        for (var feature of config["features"]) {
          if (feature["id"] === undefined) {
            feature["id"] = uuid.v4();
          }
        }
      }
    }
  }
}

for (var protocol in doc["protocols"]) {
  console.log(protocol);
  if (doc["protocols"][protocol]["defaults"] !== undefined) {
    for (var feature of doc["protocols"][protocol]["defaults"]["features"]) {
      if (feature["actuator"] !== undefined) {
        let act = {... feature["actuator"] } ;
        let feature_type = feature["feature-type"];
        if (act["messages"].includes("LinearCmd")) {
          feature["feature-type"] = "PositionWithDuration"
          feature_type = "PositionWithDuration"
        }
        if (act["messages"].includes("RotateCmd")) {
          feature["feature-type"] = "RotateWithDirection"
          feature_type = "RotateWithDirection"
        }
        feature["output"] = {};
        feature["output"][feature_type] = {
          "step-range": act["step-range"]
        };
        delete feature["actuator"];
      }
      if (feature["sensor"] !== undefined) {
        let sen = {... feature["sensor"]};
        feature["input"] = {};        
        feature["input"][feature["feature-type"]] = {
          "value-range": sen["value-range"],
          "input-commands": ["Read"]
        };
        delete feature["sensor"]
      }
    }
  }
  if (doc["protocols"][protocol]["configurations"] !== undefined) {
    for (var config of doc["protocols"][protocol]["configurations"]) {
      if (config["features"] === undefined) continue;
      for (var feature of config["features"]) {
        if (feature["actuator"] !== undefined) {
          let act = {... feature["actuator"]};
          let feature_type = feature["feature-type"];
          if (act["messages"].includes("LinearCmd")) {
            feature["feature-type"] = "PositionWithDuration"
            feature_type = "PositionWithDuration"
          }
          if (act["messages"].includes("RotateCmd")) {
            feature["feature-type"] = "RotateWithDirection"
            feature_type = "RotateWithDirection"
          }          
          feature["output"] = {};
          feature["output"][feature_type] = {
            "step-range": act["step-range"]
          };
          delete feature["actuator"];
        }
        if (feature["sensor"] !== undefined) {
          let sen = {... feature["sensor"]};
          feature["input"] = {};        
          feature["input"][feature["feature-type"]] = {
            "value-range": sen["value-range"],
            "input-commands": ["Read"]
          };
          delete feature["sensor"]
        }
      }
    }
  }
  fs.writeFileSync(`device-config-v4/protocols/${protocol}.yml`, yaml.dump(doc["protocols"][protocol]));
}
