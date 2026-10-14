"""Golden hit and bounds checks for the installed 2009 Livewire movie."""

import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

# Values are small assertions, not a copy of the client art or atlas.
GOLDEN = {
    'gBlackGreen1': ((410, 15), (-6, 820, -6, 42)),
    'gBlackGreen2': ((415, 25), (-6, 814, -25, 40)),
    'gBlackGreen3': ((370, 15), (-3, 826, -70, 32)),
    'gBlackGreen4': ((415, 15), (-6, 820, -3, 25)),
    'gGreenBlack1': ((440, -55), (-5, 892, -93, 20)),
    'gGreenBlack2': ((410, 75), (-5, 833, -24, 81)),
    'gGreenBlack3': ((425, 5), (-12, 832, -17, 78)),
    'gGreenBlack4': ((440, -40), (-4, 893, -179, 82)),
    'gRedSilver1': ((535, 35), (-15, 995, -34, 52)),
    'gRedSilver2': ((425, -5), (-7, 859, -38, 26)),
    'gRedSilver3': ((510, 0), (-8, 1036, -36, 72)),
    'gRedSilver4': ((420, 0), (-9, 859, -79, 56)),
    'mBlackStripe1': ((405, 0), (-8, 828, -47, 48)),
    'mBlackStripe2': ((390, 20), (-4, 826, -66, 46)),
    'mBlackStripe3': ((425, -40), (-7, 869, -60, 88)),
    'mBlackStripe4': ((405, -125), (-17, 818, -210, 28)),
    'mGear': ((0, 5), (-94, 94, -94, 94)),
    'mMicroChip1': ((5, 15), (-35, 37, -27, 30)),
    'mMicroChip2': ((0, -5), (-43, 43, -31, 31)),
    'mOrangeStripe1': ((450, -95), (-12, 946, -184, 98)),
    'mOrangeStripe2': ((405, 25), (-12, 833, -24, 36)),
    'mOrangeStripe3': ((430, -10), (-6, 905, -113, 16)),
    'mOrangeStripe4': ((405, 55), (-11, 833, -8, 136)),
    'mPurpleStripe1': ((475, -50), (-7, 997, -100, 61)),
    'mPurpleStripe2': ((480, 25), (-9, 871, -92, 92)),
    'mPurpleStripe3': ((425, -30), (-25, 871, -48, 59)),
    'mPurpleStripe4': ((440, 20), (-8, 891, -71, 61)),
    'mYellowStripe1': ((425, 0), (-26, 888, -16, 62)),
    'mYellowStripe2': ((380, -30), (-15, 942, -74, 64)),
    'mYellowStripe3': ((445, -5), (-10, 856, -98, 75)),
    'mYellowStripe4': ((425, -40), (-10, 857, -72, 26)),
    'oBlack1': ((445, 0), (-10, 883, -24, 50)),
    'oBlack2': ((455, -25), (-9, 891, -44, 64)),
    'oBlack3': ((435, -10), (-10, 883, -28, 52)),
    'oBlack4': ((435, 55), (-13, 878, -19, 180)),
    'oGreen1': ((410, -10), (-9, 871, -40, 60)),
    'oGreen2': ((395, 30), (-3, 846, -5, 36)),
    'oGreen3': ((415, -25), (-8, 838, -71, 74)),
    'oGreen4': ((415, 45), (-10, 840, -49, 122)),
    'oRed1': ((420, 20), (-2, 840, -26, 36)),
    'oRed2': ((425, 70), (-26, 881, -26, 113)),
    'oRed3': ((395, -5), (-12, 830, -53, 75)),
    'oRed4': ((420, -10), (-7, 835, -92, 36)),
    'pBronze1': ((430, 35), (0, 878, -11, 84)),
    'pBronze2': ((420, 5), (-4, 868, -28, 39)),
    'pBronze3': ((435, 45), (-11, 879, -10, 56)),
    'pBronze4': ((435, -30), (-11, 878, -98, 30)),
    'pGray1': ((395, -95), (-7, 869, -99, 79)),
    'pGray2': ((450, -30), (-8, 873, -37, 64)),
    'pGray3': ((435, 55), (-4, 916, -31, 75)),
    'pGray4': ((500, -10), (-9, 994, -97, 35)),
    'pGreen1': ((450, 5), (-7, 923, -49, 81)),
    'pGreen2': ((345, 95), (-6, 832, -45, 108)),
    'pGreen3': ((440, 40), (-4, 872, -4, 51)),
    'pGreen4': ((440, 40), (-6, 874, -24, 84)),
    'pLEDBlue': ((0, 0), (-23, 25, -24, 24)),
    'pLEDGreen': ((0, 0), (-23, 25, -24, 24)),
    'pLEDRed': ((0, 0), (-25, 23, -24, 24)),
    'pRust1': ((440, 15), (-4, 905, -13, 39)),
    'pRust2': ((445, 5), (-1, 917, -10, 37)),
    'pRust3': ((440, 40), (-1, 942, -15, 61)),
    'pRust4': ((485, 50), (-1, 941, -19, 119)),
}

class AtlasGoldenTest(unittest.TestCase):
    def test_installed_movie(self):
        upk = os.environ.get("CIMMERIA_LIVEWIRE_UPK")
        if not upk:
            self.skipTest("set CIMMERIA_LIVEWIRE_UPK to the installed Livewire.upk")
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "atlas.json"
            subprocess.run([sys.executable, str(Path(__file__).with_name("atlas.py")), upk, str(output)], check=True)
            atlas = json.loads(output.read_text(encoding="utf-8"))
        self.assertEqual(atlas["version"], 1)
        self.assertEqual(atlas["step"], 5)
        self.assertEqual(set(atlas["libraries"]), set(GOLDEN))
        for name, (hit_point, bounds) in GOLDEN.items():
            with self.subTest(name=name):
                lib = atlas["libraries"][name]
                self.assertIn(list(hit_point), lib["points"])
                self.assertEqual(tuple(round(v / 20) for v in lib["bounds_twips"]), bounds)

if __name__ == "__main__":
    unittest.main()
