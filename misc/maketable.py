import pandas as pd

d = pd.concat([pd.read_csv("benchresults.csv"), pd.read_csv("benchresults_copar.csv")])

d['type'] = d['file'].str.extract(".*/.*/([a-zA-Z_]*[a-zA-Z])")

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