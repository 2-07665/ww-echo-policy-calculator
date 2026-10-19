"""Check snapshot coverage, sparse inheritance, and source coefficient conversion."""
import json
import re
import unittest
from decimal import Decimal, ROUND_HALF_UP
from pathlib import Path
from generate_wuwa_echo_tool_presets import BUFF_NAMES, ROLE_NAME_ALIASES, generate_presets

ROOT = Path(__file__).resolve().parents[2]
SOURCE = json.loads((Path(__file__).parent / 'sources/wuwa_echo_tool.json').read_text())
OUTPUT = json.loads((ROOT / 'crates/echo_app/presets/wuwa_echo_tool.json').read_text())

class Presets(unittest.TestCase):
    def test_reproducible_and_complete(self):
        self.assertEqual(generate_presets(SOURCE), OUTPUT)
        self.assertEqual(len(OUTPUT['presets']), 60)
        self.assertEqual(sum(len(p['variants']) for p in OUTPUT['presets']), 284)
        groups = {p['presetName']:p for p in OUTPUT['presets']}
        # Display-unit maxima; independently compare all modes/chains to source coefficients.
        maxima = ['10.5','21','11.6','14.7','11.6','60','70','580','12.4','11.6','11.6','11.6','11.6']
        for role in SOURCE['roles']:
            group = groups[ROLE_NAME_ALIASES.get(role['name'],role['name'])]
            for mode in role['modes']:
                suffix = '' if mode['mode']=='default' else ' · '+SOURCE['modes'][mode['mode']]['zh']
                for profile in mode['profiles']:
                    matches=[]
                    for variant in group['variants']:
                        match=re.fullmatch(r'(\d)(?:-(\d))?链'+re.escape(suffix),variant['variantName'])
                        if match and int(match[1]) <= profile['chain'] <= int(match[2] or match[1]):
                            matches.append(variant)
                    self.assertEqual(len(matches),1,(role['name'],mode['mode'],profile['chain']))
                    weights=group['variants'][0]['weights'] | matches[0]['weights']
                    for name,maximum,coefficient in zip(BUFF_NAMES,maxima,profile['coefficients']):
                        expected=(Decimal(str(coefficient))*Decimal(maximum)*10).quantize(Decimal('.01'),rounding=ROUND_HALF_UP)
                        self.assertEqual(Decimal(str(weights[name])),expected,(role['name'],profile['chain'],name))

    def test_runtime_modes_and_legacy_identity(self):
        groups={p['presetName']:p for p in OUTPUT['presets']}
        self.assertEqual(OUTPUT['presets'][0]['presetName'],'今汐')
        self.assertEqual(OUTPUT['presets'][0]['variants'][1]['weights']['Skill_Damage'],73.31)
        self.assertEqual(groups['雷主']['variants'][0]['weights']['Skill_Damage'],72.5)
        self.assertEqual(groups['穗穗']['variants'][0]['weights']['Crit_Rate'],0)
        self.assertEqual(groups['穗穗']['variants'][0]['weights']['ER'],148.8)
        self.assertTrue(any('聚爆' in v['variantName'] for v in groups['爱弥斯']['variants']))
        self.assertTrue(any('集谐' in v['variantName'] for v in groups['达妮娅']['variants']))
        self.assertGreater(groups['景燃']['variants'][0]['weights']['HP'],0)
        self.assertTrue(any('电磁' in v['variantName'] for v in groups['心']['variants']))

if __name__=='__main__': unittest.main()
