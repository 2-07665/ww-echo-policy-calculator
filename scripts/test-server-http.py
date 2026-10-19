"""Exercise a release server in an isolated process. No external dependencies.
Usage: python3 scripts/test-server-http.py /path/to/echo_server
"""
import concurrent.futures
import hashlib
import json
import os
import socket
import statistics
import subprocess
import sys
import time
import urllib.error
import urllib.request

binary = sys.argv[1] if len(sys.argv) > 1 else 'target/release/echo_server'
# Exercise the real generation command without printing or saving its secret.
generated = subprocess.check_output([binary, 'generate-key'], text=True)
key = generated.splitlines()[1]
digest = generated.split('ECHO_ADMIN_KEY_HASHES=')[1].strip()
assert digest == hashlib.sha256(key.encode()).hexdigest()
assert key != subprocess.check_output([binary, 'generate-key'], text=True).splitlines()[1]
with socket.socket() as sock:
    sock.bind(('127.0.0.1', 0))
    port = sock.getsockname()[1]
env = dict(os.environ, ECHO_BIND=f'127.0.0.1:{port}', ECHO_TRUSTED_PROXIES='127.0.0.1',
           ECHO_ADMIN_KEY_HASHES=digest, ECHO_QUOTA_JOINT='1', ECHO_REQUESTS_PER_MINUTE='10000')
process = subprocess.Popen([binary], env=env, stdout=subprocess.DEVNULL)
base = f'http://127.0.0.1:{port}'
def call(path, body=None, ip='192.0.2.1', token=None):
    headers = {'X-Forwarded-For': ip}
    if token: headers['Authorization'] = 'Bearer ' + token
    if body is not None: headers['Content-Type'] = 'application/json'
    req = urllib.request.Request(base + '/api/' + path, data=None if body is None else json.dumps(body).encode(), headers=headers)
    try:
        with urllib.request.urlopen(req, timeout=30) as r: return r.status, json.load(r), dict(r.headers)
    except urllib.error.HTTPError as e: return e.code, json.load(e), dict(e.headers)
def finish(job, ip='192.0.2.1'):
    deadline = time.monotonic() + 30
    while time.monotonic() < deadline:
        status, data, _ = call('jobs/' + job['id'], ip=ip)
        assert status == 200, data
        if data['status'] == 'done': return data['result']
        assert data['status'] != 'failed', data
        time.sleep(.01)
    raise AssertionError('job timeout')
try:
    for _ in range(200):
        try:
            _, boot, _ = call('bootstrap'); break
        except OSError: time.sleep(.025)
    else: raise AssertionError('server did not start')
    config = boot['defaults']
    request = {'config': config, 'echo': []}
    code, first, _ = call('solve', request); assert code == 202
    with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:
        jobs = list(pool.map(lambda i: (i,call('solve',request,ip=f'192.0.2.{i}')),range(2,6)))
    assert call('jobs/'+first['id'],ip='192.0.2.99')[0] == 404
    finish(first)
    for i,(status,job,_) in jobs:
        assert status == 202; finish(job,f'192.0.2.{i}')
    other = {'config': dict(config,target=71), 'echo': []}
    code, failure, headers = call('solve',other); assert code == 429 and failure['code'] == 'quota_exceeded'
    assert any(k.lower() == 'retry-after' for k in headers)
    assert call('solve',other,ip='198.51.100.100, 192.0.2.1')[0] == 429, 'spoofed prefix bypassed quota'
    assert call('solve',other,token='wrong')[0] == 401
    assert call('auth',{},token=key)[0] == 200
    code, job, _ = call('solve',other,token=key); assert code == 202; finish(job)
    # A larger cold joint solve must not block existing cached queries.
    heavier = dict(config, weights=[100,100,70,30,0,36,30,0,40,0,0,0,0])
    code, cold, _ = call('solve', {'config':heavier,'echo':[]}, token=key); assert code == 202
    times=[]
    for _ in range(50):
        start=time.monotonic()
        code, job, _ = call('solve',request)
        assert code == 202
        assert finish(job)['cached']
        times.append((time.monotonic()-start)*1000)
    finish(cold)
    memory=[]
    try:
        with open(f'/proc/{process.pid}/status') as f:
            memory=[line.strip() for line in f if line.startswith(('VmRSS:','VmHWM:'))]
    except FileNotFoundError: pass
    print(json.dumps({'passed': True, 'clients':5, 'cached_queries':len(times), 'query_p50_ms':round(statistics.median(times),2), 'query_p95_ms':round(sorted(times)[47],2), 'memory':memory}))
finally:
    process.terminate()
    try: process.wait(timeout=5)
    except subprocess.TimeoutExpired: process.kill(); process.wait()
