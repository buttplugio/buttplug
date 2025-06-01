const yaml = require('js-yaml');
const uuid = require('uuid');
const fs = require('fs');

// Get document, or throw exception on error
const doc = yaml.load(fs.readFileSync('./device-config-v4/buttplug-device-config-v4.yml', 'utf8'));
for (var protocol in doc["protocols"]) {
  console.log(protocol);
  if (doc["protocols"][protocol]["defaults"] !== undefined) {
    for (var feature of doc["protocols"][protocol]["defaults"]["features"]) {
      if (feature["actuator"] !== undefined) {
        let act = {... feature["actuator"] } ;
        feature["actuator"] = {};
        feature["actuator"][feature["feature-type"]] = act;
      }
      if (feature["sensor"] !== undefined) {
        let sen = {... feature["sensor"]};
        feature["sensor"] = {};        
        feature["sensor"][feature["feature-type"]] = sen;
      }
    }
  }
  if (doc["protocols"][protocol]["configurations"] !== undefined) {
    for (var config of doc["protocols"][protocol]["configurations"]) {
      if (config["features"] === undefined) continue;
      for (var feature of config["features"]) {
        if (feature["actuator"] !== undefined) {
          let act = {... feature["actuator"]};
          feature["actuator"] = {};
          feature["actuator"][feature["feature-type"]] = act;
        }
        if (feature["sensor"] !== undefined) {
          let sen = {... feature["sensor"]};
          feature["sensor"] = {};        
          feature["sensor"][feature["feature-type"]] = sen;
        }
      }
    }
  }
}

fs.writeFileSync("device-config-v4/buttplug-device-config-v4-new.yml", yaml.dump(doc));
