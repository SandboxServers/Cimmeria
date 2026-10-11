// One `npm run uat:adoption` on every host: macOS makes a verified copy, every
// other host adopts the imported folder in place (D-LX12).
await import(process.platform==='darwin'?'./adoption-native-uat.mjs':'./adoption-in-place-uat.mjs');
