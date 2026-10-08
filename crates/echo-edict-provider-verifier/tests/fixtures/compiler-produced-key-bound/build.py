# SPDX-License-Identifier: Apache-2.0
# Reproduce the bounded-key public application build inside the guarded worker.
import hashlib,json,shutil,subprocess
from pathlib import Path

def enc(v):
 def head(m,n):
  if n<24:return bytes([m*32+n])
  for a,k in [(24,1),(25,2),(26,4),(27,8)]:
   if n<1<<(8*k):return bytes([m*32+a])+n.to_bytes(k,'big')
  raise ValueError('large integer')
 if v is None:return b'\xf6'
 if isinstance(v,bool):return b'\xf5' if v else b'\xf4'
 if isinstance(v,int):return head(0,v) if v>=0 else head(1,-1-v)
 if isinstance(v,str):
  b=v.encode();return head(3,len(b))+b
 if isinstance(v,bytes):return head(2,len(v))+v
 if isinstance(v,list):return head(4,len(v))+b''.join(map(enc,v))
 if isinstance(v,dict):
  items=sorted([(enc(k),enc(x)) for k,x in v.items()],key=lambda p:(len(p[0]),p[0]))
  return head(5,len(items))+b''.join(k+x for k,x in items)
 raise ValueError(type(v))
def dec(b):
 i=0
 def item():
  nonlocal i
  h=b[i];i+=1;m=h>>5;a=h&31
  if a>=24:
   assert a<=27
   n=1<<(a-24);a=int.from_bytes(b[i:i+n],'big');i+=n
  if m==0:return a
  if m==1:return -1-a
  if m in (2,3):
   v=b[i:i+a];i+=a;return v if m==2 else v.decode()
  if m==4:return [item() for _ in range(a)]
  if m==5:return {item():item() for _ in range(a)}
  if m==7:return {20:False,21:True,22:None}[a]
  raise ValueError(m)
 v=item();assert i==len(b) and enc(v)==b;return v

def digest(domain,value):return hashlib.sha256(enc(['edict.digest/v1',domain,value])).digest()
import argparse
parser=argparse.ArgumentParser()
parser.add_argument('--compiler',type=Path,required=True)
parser.add_argument('--compiler-sha256',required=True)
parser.add_argument('--provider-package',type=Path,required=True)
parser.add_argument('--output-directory',type=Path,required=True)
args=parser.parse_args()
root=args.output_directory.resolve();root.mkdir(exist_ok=False)
fixture=Path(__file__).resolve().parent
compiler=args.compiler.resolve()
assert hashlib.sha256(compiler.read_bytes()).hexdigest()==args.compiler_sha256
print('COMPILER_BINARY_SHA256',args.compiler_sha256,flush=True)
config=dec((fixture/'echo-operation-configuration.cbor').read_bytes());config['apiVersion']='echo.operation-lowering-configuration/v2'
adapter=dec((fixture/'adapter.cbor').read_bytes())
for impl in adapter['effectImplementations'].values():
 ref=impl['targetConfiguration'];ref['id']='echo.operation-lowering-configuration/v1';ref['digest']=['sha256',digest(ref['id'],config)]
manifest=dec((fixture/'manifest.cbor').read_bytes())
for selection in manifest['targetAdapters']:
 ref=selection['adapter'];ref['digest']=['sha256',digest(ref['id'],adapter)]
for name,value in [('echo-operation-configuration.cbor',config),('adapter.cbor',adapter),('manifest.cbor',manifest)]: (root/name).write_bytes(enc(value))
shutil.copyfile(fixture/'exports.cbor',root/'exports.cbor')
source=(fixture/'create-greeting.edict').read_text()
old=digest('edict.lawpack/v1',dec((fixture/'manifest.cbor').read_bytes())).hex()
assert old in source
(root/'create-greeting.edict').write_text(source.replace(old,digest('edict.lawpack/v1',manifest).hex()))
shutil.copytree(args.provider_package.resolve(),root/'provider')
app={'schema':'edict.application/v1','coordinate':'examples.hello_echo@1','sources':['create-greeting.edict'],'lawpacks':[{'manifest':'manifest.cbor','exports':'exports.cbor','adapter':'adapter.cbor','targetConfiguration':'echo-operation-configuration.cbor'}],'target':{'profile':'echo.dpo@1','providerPackage':'provider'},'outputDirectory':'build'}
(root/'edict.application.json').write_text(json.dumps(app))
request={'schema':'edict.compiler.settings/v1','type':'compilerSettings','operation':'build','application':str(root/'edict.application.json')}
result=subprocess.run([str(compiler)],input=json.dumps(request)+'\n',text=True,capture_output=True,timeout=90)
assert len(result.stdout)+len(result.stderr)<128*1024
(root/'compiler.stdout.jsonl').write_text(result.stdout);(root/'compiler.stderr.txt').write_text(result.stderr)
print(result.stdout,result.stderr,flush=True)
assert result.returncode==0,result.returncode
print('OUTPUT_FILES',sorted(str(p.relative_to(root)) for p in (root/'build').rglob('*') if p.is_file()),flush=True)
