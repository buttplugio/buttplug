const yaml = require('js-yaml');
const uuid = require('uuid');
const fs = require('fs');

let doc = {
  version: {
    major: 4,
    minor: 0
  },
  protocols: {}
};

for (var protocol_file of fs.readdirSync('./device-config-v4/protocols')) {
  console.log(protocol_file);
  let protocol_name = protocol_file.split(".")[0];
  let protocol = yaml.load(fs.readFileSync(`./device-config-v4/protocols/${protocol_file}`, 'utf8'));
  doc.protocols[protocol_name] = protocol;
}

fs.writeFileSync('./build-config/buttplug-device-config-v4.json', JSON.stringify(doc));
