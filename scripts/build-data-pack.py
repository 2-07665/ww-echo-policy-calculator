#!/usr/bin/env python3
"""Build the deployable pack from weight sources and curated bilingual metadata."""
import argparse
import copy
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SOURCES = {'linear':'linear_default', 'fixed':'fixed', 'wuwa_echo_tool':'wuwa_echo_tool',
           'mc_boost_assistant':'mc_boost_assistant', 'qq_bot':'qq_bot'}

def build(version, catalog, sources, previous):
    # Preserve legacy IDs across source reordering. New identities use a semantic hash.
    old = {(scoring,g['presetName'],v['variantName']):v['id']
           for scoring,source in previous.get('presets',{}).items()
           for g in source['presets'] for v in g['variants']}
    pack = {'schema_version':1, 'version':version, **copy.deepcopy(catalog), 'presets':{}}
    for scoring, source in sources.items():
        source=copy.deepcopy(source)
        for group in source['presets']:
            if group['presetName'] not in catalog['characters']:
                raise ValueError(f"Missing character metadata: {group['presetName']}")
            for variant in group['variants']:
                name=variant.setdefault('variantName','默认')
                if name not in catalog['variant_names']:
                    raise ValueError(f'Missing variant translation: {name}')
                intro=variant.get('presetIntro')
                if intro and intro not in catalog['descriptions']:
                    raise ValueError(f'Missing preset description translation: {intro}')
                identity=(scoring,group['presetName'],name)
                new_id='data:'+hashlib.sha256(json.dumps(identity,ensure_ascii=False).encode()).hexdigest()[:24]
                variant['id']=variant.get('id') or old.get(identity) or new_id
        pack['presets'][scoring]=source
    return pack

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--version',required=True,help='New data release version, e.g. 2026.10.20.1')
    parser.add_argument('--output',type=Path,default=ROOT/'data/echo-data.json')
    parser.add_argument('--check',action='store_true',help='Verify the committed pack is reproducible')
    args=parser.parse_args()
    previous=json.loads((ROOT/'data/echo-data.json').read_text())
    catalog=json.loads((ROOT/'data/catalog.json').read_text())
    sources={key:json.loads((ROOT/f'crates/echo_app/presets/{name}.json').read_text()) for key,name in SOURCES.items()}
    pack=build(args.version,catalog,sources,previous)
    output=json.dumps(pack,ensure_ascii=False,indent=2)+'\n'
    if args.check:
        if output!=args.output.read_text(): raise SystemExit('Data pack is stale; regenerate it.')
        print('Data pack is reproducible.')
    else:
        args.output.write_text(output)
        print(f'Wrote {args.output} ({len(output.encode())} bytes)')

if __name__=='__main__': main()
