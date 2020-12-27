'use strict';

const fs = require('fs');

let rawdata = fs.readFileSync('./buttplug-device-config.json');
let jsondata = JSON.parse(rawdata);
fs.writeFileSync('version', jsondata["version"].toString());