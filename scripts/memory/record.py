# Every second for LIMIT seconds after the window opens: each process's PSS,
# and the web process by kind. Writes a CSV. Usage: record.py OUT.csv [PROCESS] [LIMIT]
import os, re, time, collections, sys
out = open(sys.argv[1], 'w')
MAIN = sys.argv[2] if len(sys.argv) > 2 else 'wallpicker'
LIMIT = float(sys.argv[3]) if len(sys.argv) > 3 else 120
def kids():
    def comm_of(p):
        try: return open(f'/proc/{p}/comm').read().strip()
        except OSError: return ''
    tray = [p for p in os.listdir('/proc') if p.isdigit() and comm_of(p) == MAIN]
    res = {}
    for p in os.listdir('/proc'):
        if not p.isdigit(): continue
        try:
            st = open(f'/proc/{p}/stat').read().rsplit(')', 1)[1].split()
            comm = open(f'/proc/{p}/comm').read().strip()
        except OSError: continue
        if comm == MAIN or (st[1] in tray and comm.startswith('WebKit')):
            res[comm] = int(p)
    return res
def pss(p):
    try:
        for l in open(f'/proc/{p}/smaps_rollup'):
            if l.startswith('Pss:'): return int(l.split()[1]) // 1024
    except OSError: return 0
def kinds(p):
    k = collections.Counter(); name = None
    try:
        for l in open(f'/proc/{p}/smaps'):
            m = re.match(r'^[0-9a-f]+-[0-9a-f]+ \S+ \S+ \S+ \S+\s*(.*)$', l)
            if m:
                n = m.group(1)
                name = ('anon' if n == '' else 'heap' if n == '[heap]' else 'gpu' if (n.startswith('/dev/dri') or 'dmabuf' in n or 'gbm' in n or 'drm' in n)
                        else 'memfd' if n.startswith('/memfd') else 'libs' if '.so' in n else 'other')
            elif l.startswith('Pss:'): k[name] += int(l.split()[1]) // 1024
    except OSError: pass
    return k
while 'WebKitWebProces' not in kids(): time.sleep(0.5)
out.write('secs,total,main,network,web,web_anon,web_heap,web_gpu,web_memfd,web_libs,web_other\n')
start = time.time()
while time.time() - start < LIMIT:
    ks = kids(); w = ks.get('WebKitWebProces')
    main, net, web = pss(ks.get(MAIN, 0)), pss(ks.get('WebKitNetworkPr', 0)) if 'WebKitNetworkPr' in ks else 0, pss(w) if w else 0
    k = kinds(w) if w else {}
    out.write(f"{time.time()-start:.0f},{main+net+web},{main},{net},{web},{k.get('anon',0)},{k.get('heap',0)},{k.get('gpu',0)},{k.get('memfd',0)},{k.get('libs',0)},{k.get('other',0)}\n"); out.flush()
    time.sleep(1)
