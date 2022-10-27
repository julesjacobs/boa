import pandas as pd
import math
from statistics import mean

d = pd.concat([pd.read_csv("benchresultsmcrl2_all.csv"), pd.read_csv("benchresults.csv")])

d['type'] = d['file'].str.extract(".*/.*/([a-zA-Z_]*[a-zA-Z])")

results = dict()

for index,row in d.iterrows():
  key = row.file[0:-4]
  if key not in results:
    results[key] = dict()
  res = results[key]
  alg = str(row.algorithm)
  if alg not in res:
    res[alg] = {'time': [], 'mem': []}
  if alg == "nlogn":
    res[alg]['time'].append(row.time_sec)
  else:
    res[alg]['time'].append(row.selfreport)
  res[alg]['mem'].append(row.mem_mb)
  res['type'] = row.type
  if not math.isnan(row.num_states):
    res['num_states'] = int(row.num_states)
  if not math.isnan(row.num_states_min):
    res['num_states_min'] = int(row.num_states_min)

keysbytime = sorted(results.keys(), key=lambda k: (results[k]['type'],results[k]['bisim']['time']))
algs = ['bisim', 'nlogn']
for file in keysbytime:
  res = results[file]
  bisim = res['bisim']
  nlogn = res['nlogn']
  if mean(bisim['time']) < 1: continue
  nums = [
    res['type'],
    str(res['num_states']),
    str(res['num_states_min']),
    str(round(mean(bisim['time']), 1)),
    str(round(mean(nlogn['time']), 1)),
    str(round(mean(bisim['mem']))),
    str(round(mean(nlogn['mem'])))
  ]
  print("\t\t&\t\t".join(nums))
  # print(file, nums)


exit()

df = d.pivot(index='file', columns='algorithm', values=['time_sec', 'mem_mb', 'compressedsize_mb', 'num_states', 'type'])

sizes = df['compressedsize_mb']['nlogn']
df.drop([('compressedsize_mb')], axis=1,inplace=True)
df['compressedsize_mb'] = sizes

num_states = df['num_states']['nlogn']
df.drop([('num_states')], axis=1,inplace=True)
df['num_states'] = num_states

sizes = df['type']['nlogn']
df.drop([('type')], axis=1,inplace=True)
df['type'] = sizes

col = df.pop('type'); df.insert(0, col.name, col)
col = df.pop('num_states'); df.insert(1, col.name, col)
col = df.pop('compressedsize_mb'); df.insert(2, col.name, col)

df.drop([('mem_mb','copar')], axis=1,inplace=True)

df.sort_values(['type','num_states'],inplace=True)
df.set_index(['type', 'num_states', 'compressedsize_mb'], inplace=True)
# df.sort_index(inplace=True)
# df.set_index(['file'], inplace=True)
df.columns.names = [None,None]

df = df.rename(index = lambda x: round(x,1) if isinstance(x,float) else x)
df = df.apply(lambda x: round(x,1) if isinstance(x,float) else x)

for col in df.columns:
  df[col] = df[col].apply(lambda x: round(x,2))

open('benchtable.html','w').write(df.to_html(index_names=False)) # = ['type','n','compressed size (MB)']
open('benchtable.tex','w').write(df.to_latex())

# print(p['time_sec'])

# for r in d.rows():
  # print(r)
  # print(r['file'], r['time_sec']['copar'])