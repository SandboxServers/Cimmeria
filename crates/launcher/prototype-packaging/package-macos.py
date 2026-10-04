"""Package unsigned/ad-hoc developer proofs, never a notarized release."""
from pathlib import Path
import plistlib
import shutil
import subprocess
root = Path(__file__).resolve().parents[3]
proof = Path(__file__).resolve().parent
dist = proof / 'dist'
dist.mkdir(exist_ok=True)
app = dist / 'Cimmeria Egui Proof.app'
(app / 'Contents/MacOS').mkdir(parents=True, exist_ok=True)
(app / 'Contents/Resources').mkdir(exist_ok=True)
shutil.copy2(root / 'target/release/cimmeria-egui-proof', app / 'Contents/MacOS/cimmeria-egui-proof')
with (app / 'Contents/Info.plist').open('wb') as f:
    plistlib.dump(dict(CFBundleExecutable='cimmeria-egui-proof', CFBundleIdentifier='app.cimmeria.packaging.egui', CFBundleName='Cimmeria Egui Proof', CFBundlePackageType='APPL', CFBundleVersion='1', CFBundleShortVersionString='0.1.0', NSHighResolutionCapable=True), f)
subprocess.run(['codesign', '--force', '--sign', '-', str(app)], check=True)
tauri = root / 'target/release/bundle/macos/Cimmeria Tauri Proof.app'
if tauri.exists():
    subprocess.run(['ditto', str(tauri), str(dist / tauri.name)], check=True)
for bundle in dist.glob('*.app'):
    subprocess.run(['ditto', '-c', '-k', '--sequesterRsrc', '--keepParent', str(bundle), str(dist / (bundle.stem + '.zip'))], check=True)
print(dist)
