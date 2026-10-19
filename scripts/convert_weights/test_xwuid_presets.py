"""Validate generated presets against the pinned upstream data (offline)."""
import json
import unittest
from decimal import Decimal

from generate_qq_bot_presets import build_presets, DEFAULT_INPUT_SNAPSHOT, DEFAULT_OUTPUT_JSON

D = lambda value: Decimal(str(value))


class XwuidPresetsTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.source = json.loads(DEFAULT_INPUT_SNAPSHOT.read_text())
        cls.output = json.loads(DEFAULT_OUTPUT_JSON.read_text())
        cls.variants = [v for g in cls.output['presets'] for v in g['variants']]

    def test_reproducible_and_complete(self):
        self.assertEqual(build_presets(DEFAULT_INPUT_SNAPSHOT), self.output)
        self.assertEqual(len(self.output['presets']), 61)
        self.assertEqual(len(self.variants), 278)
        self.assertEqual(len({v['id'] for v in self.variants}), 278)
        # Every source mode is represented, including both identical Rover forms.
        for character, modes in self.source['templates'].items():
            for modal, template in modes.items():
                represented = [v for v in self.variants if
                    self.source['templates'][v['sourceTemplate']['characterId']][v['sourceTemplate']['modal']] == template]
                self.assertTrue(represented, (character, modal))
                self.assertEqual({v['cost'] for v in represented}, {1, 3, 4})
                for cost in [1, 3, 4]:
                    fixed = '生命' if cost == 1 else '攻击'
                    expected = {k for k, value in template['main_props'][str(cost)].items() if value > 0 and k != fixed}
                    actual = {name for v in represented if v['cost'] == cost for name in v['mainStats']}
                    self.assertEqual(actual, expected)

    def test_all_weights_and_main_scores_match_source(self):
        direct = {'Crit_Rate':'暴击', 'Crit_Damage':'暴击伤害', 'Attack':'攻击%',
                  'Defence':'防御%', 'HP':'生命%', 'Attack_Flat':'攻击',
                  'Defence_Flat':'防御', 'HP_Flat':'生命', 'ER':'共鸣效率'}
        skills = ['Basic_Attack_Damage', 'Heavy_Attack_Damage', 'Skill_Damage', 'Ult_Damage']
        main_values = {
            1: {'攻击%':18, '生命%':22.8, '防御%':18},
            3: {'攻击%':30, '生命%':30, '防御%':38, '共鸣效率':32, '属性伤害加成':30},
            4: {'攻击%':33, '生命%':33, '防御%':41.8, '暴击':22, '暴击伤害':44, '治疗效果加成':26.4},
        }
        for v in self.variants:
            origin = v['sourceTemplate']
            template = self.source['templates'][origin['characterId']][origin['modal']]
            sub = template['sub_props']
            expected = {dest:D(sub.get(src, 0)) for dest, src in direct.items()}
            expected.update({dest:D(sub.get('技能伤害加成', 0))*D(template['skill_weight'][i]) for i, dest in enumerate(skills)})
            self.assertEqual({k:D(w) for k,w in v['weights'].items()}, expected, v['id'])
            cost = v['cost']
            main = template['main_props'][str(cost)]
            fixed = D(main.get('生命' if cost == 1 else '攻击', 0))*D({1:2280,3:100,4:150}[cost])
            for name in v['mainStats']:
                self.assertEqual(D(v['mainBuffScore']), D(main[name])*D(main_values[cost][name])+fixed, v['id'])
            self.assertEqual(v['sourceScoreMax'], template['score_max'][[1,3,4].index(cost)])

    def test_changed_jinhsi_weights_and_lucilla_modes(self):
        jinhsi = next(g for g in self.output['presets'] if g['presetName'] == '今汐')
        self.assertEqual(jinhsi['variants'][0]['weights']['ER'], .25)
        self.assertEqual(jinhsi['variants'][0]['weights']['Skill_Damage'], .715)
        lucilla = next(g for g in self.output['presets'] if g['presetName'] == '洛瑟菈')
        modes = {v['sourceTemplate']['modal']:v['weights'] for v in lucilla['variants']}
        self.assertEqual(set(modes), {'default','featherfall','phantom','phantom-featherfall'})
        self.assertNotEqual(modes['default'], modes['phantom'])
        self.assertNotEqual(modes['default'], modes['featherfall'])


if __name__ == '__main__':
    unittest.main()
