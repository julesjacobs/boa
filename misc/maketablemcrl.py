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