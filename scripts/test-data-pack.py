import copy
import importlib.util
import json
import unittest
from pathlib import Path

ROOT=Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location('builder',ROOT/'scripts/build-data-pack.py')
builder=importlib.util.module_from_spec(spec);spec.loader.exec_module(builder)
pack=json.loads((ROOT/'data/echo-data.json').read_text())
catalog=json.loads((ROOT/'data/catalog.json').read_text())
sources={key:json.loads((ROOT/f'crates/echo_app/presets/{name}.json').read_text()) for key,name in builder.SOURCES.items()}

class Pack(unittest.TestCase):
    def test_reproducible(self):
        self.assertEqual(builder.build(pack['version'],catalog,sources,pack),pack)
    def test_id_survives_reorder_and_insert(self):
        source=copy.deepcopy(sources)
        source['wuwa_echo_tool']['presets'].reverse()
        for g in source['wuwa_echo_tool']['presets']: g['variants'].reverse()
        changed=builder.build('next',catalog,source,pack)
        def identities(data):
            return {(s,g['presetName'],v['variantName']):v['id'] for s,p in data['presets'].items() for g in p['presets'] for v in g['variants']}
        self.assertEqual(identities(changed),identities(pack))
        new=copy.deepcopy(source['linear']['presets'][0]);new['presetName']='新角色'
        source['linear']['presets'].insert(0,new)
        metadata=copy.deepcopy(catalog);metadata['characters']['新角色']={'en':'New Character','release_date':'2026-11-01'}
        first=builder.build('next',metadata,source,pack)
        source['linear']['presets'].reverse()
        second=builder.build('next',metadata,source,pack)
        self.assertEqual(identities(first),identities(second))
    def test_missing_metadata_stops_release(self):
        broken=copy.deepcopy(catalog);del broken['characters']['今汐']
        with self.assertRaises(ValueError): builder.build('next',broken,sources,pack)
        broken=copy.deepcopy(catalog);del broken['variant_names']['0链']
        with self.assertRaises(ValueError): builder.build('next',broken,sources,pack)

if __name__=='__main__': unittest.main()
