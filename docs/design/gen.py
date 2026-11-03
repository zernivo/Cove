import ast
P=ast.literal_eval(open('parts.py').read())
head='<html><head><meta charset=utf-8><link rel=stylesheet href=style.css><style>{}</style></head><body><div class=desk></div>'
def page(name, extra_css, body, cc=False):
    mb=P['menubar'].replace('{cc_on}',' on' if cc else '')
    open(name,'w').write(head.format(extra_css)+mb+body+'</body></html>')

# 1. Desktop
page('s1.html','', P['dock'])

# 2. Control Centre
sl=lambda icon,v: f'<div class="sl"><span class="slt" style="width:{v}%"></span>{icon}</div>'
sun='<svg viewBox="0 0 24 24" class="si" style="color:#111"><circle cx="12" cy="12" r="4" fill="currentColor"/><g stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M12 2.5v2M12 19.5v2M2.5 12h2M19.5 12h2M5.3 5.3l1.4 1.4M17.3 17.3l1.4 1.4M5.3 18.7l1.4-1.4M17.3 6.7l1.4-1.4"/></g></svg>'
volk=P['vol'].replace('class="si"','class="si" style="color:#111"')
bt='<svg viewBox="0 0 24 24" class="si"><path d="M7 7l10 10-5 4V3l5 4L7 17" stroke="currentColor" stroke-width="1.9" fill="none" stroke-linecap="round" stroke-linejoin="round"/></svg>'
moon='<svg viewBox="0 0 24 24" class="si"><path d="M20 14.5A8 8 0 1 1 9.5 4a6.5 6.5 0 0 0 10.5 10.5z" fill="currentColor"/></svg>'
half='<svg viewBox="0 0 24 24" class="si"><circle cx="12" cy="12" r="8" stroke="currentColor" stroke-width="1.8" fill="none"/><path d="M12 4a8 8 0 0 1 0 16z" fill="currentColor"/></svg>'
cc_css='''
.cc{position:absolute;z-index:6;top:38px;right:10px;width:340px;border-radius:20px;padding:12px;display:grid;grid-template-columns:1fr 1fr;gap:10px;transform-origin:top right}
.blk{border-radius:14px;background:rgba(255,255,255,.07);padding:10px}
.net{grid-row:span 2;display:flex;flex-direction:column;gap:10px}
.row{display:flex;align-items:center;gap:10px;font-size:13px}
.row small{display:block;font-size:11.5px;color:rgba(236,232,227,.55);margin-top:1px}
.dotb{width:30px;height:30px;border-radius:50%;display:grid;place-items:center;background:rgba(255,255,255,.14);flex:none}
.dotb.on{background:#2cc9b9;color:#062824}.dotb.on .si{color:#062824}
.focus{display:flex;align-items:center;gap:10px;font-size:13px}
.mini{display:flex;gap:10px}.mini .blk{flex:1;display:flex;flex-direction:column;align-items:center;gap:6px;font-size:11.5px;text-align:center;padding:10px 4px}
.wide{grid-column:span 2}
.wide h6{font-size:12.5px;font-weight:600;margin-bottom:8px}
.sl{position:relative;height:26px;border-radius:13px;background:rgba(255,255,255,.12);overflow:hidden;display:flex;align-items:center;padding-left:6px}
.slt{position:absolute;left:0;top:0;bottom:0;background:#ece8e3;border-radius:13px}
.sl .si{position:relative;width:15px;height:15px}
.np{display:flex;align-items:center;gap:12px}
.art{width:44px;height:44px;border-radius:9px;background:url(cove-wall.png) 30% 70%/400%;flex:none}
.np b{font-size:13px;font-weight:600;display:block}.np span{font-size:12px;color:rgba(236,232,227,.55)}
.ctl{margin-left:auto;display:flex;gap:14px;align-items:center}
'''
cc=f'''<div class="cc glass">
<div class="blk net"><div class="row"><span class="dotb on">{P["wifi"]}</span><div>Wi-Fi<small>Home-5G</small></div></div>
<div class="row"><span class="dotb on">{bt}</span><div>Bluetooth<small>AirDots Pro</small></div></div>
<div class="row"><span class="dotb">{P["vol"]}</span><div>Sound output<small>Speakers</small></div></div></div>
<div class="blk focus"><span class="dotb">{moon}</span><div>Focus<small class="muted" style="display:block;font-size:11.5px">Off</small></div></div>
<div class="mini"><div class="blk"><span class="dotb on">{half}</span>Dark</div><div class="blk"><span class="dotb">{sun.replace('style="color:#111"','')}</span>Night light</div></div>
<div class="blk wide"><h6>Display</h6>{sl(sun,72)}</div>
<div class="blk wide"><h6>Sound</h6>{sl(volk,46)}</div>
<div class="blk wide np"><div class="art"></div><div><b>Night Swim</b><span>Lo-fi Focus · Spotify</span></div>
<div class="ctl"><svg viewBox="0 0 24 24" class="si"><path d="M7 5h3v14H7zM14 5h3v14h-3z" fill="currentColor"/></svg><svg viewBox="0 0 24 24" class="si"><path d="M5 5l9 7-9 7zM15 5h3v14h-3z" fill="currentColor"/></svg></div></div>
</div>'''
page('s2.html',cc_css,P['dock'].replace('app hover','app').replace('<span class=lbl>Spotify</span>','')+cc,cc=True)

# 3. Settings: your desktop is the settings
st_css='''
.win{position:absolute;z-index:6;left:50%;top:58px;transform:translateX(-50%);width:1180px;height:720px;border-radius:16px;background:#111215;border:1px solid rgba(255,255,255,.07);box-shadow:0 40px 120px rgba(0,0,0,.6);overflow:hidden}
.top{height:54px;display:flex;align-items:center;justify-content:space-between;padding:0 18px 0 20px}
.brand{display:flex;align-items:center;gap:9px;font-weight:650;font-size:15px;letter-spacing:-.01em}.brand img{width:22px}
.master{display:flex;align-items:center;gap:10px;font-size:13px;padding:6px 6px 6px 14px;border-radius:999px;background:rgba(255,255,255,.05)}
.master a{color:rgba(236,232,227,.55);text-decoration:none;padding:0 6px}
.caps{display:flex;gap:22px;color:rgba(236,232,227,.5);font-size:13px}
.tg{width:38px;height:22px;border-radius:11px;background:#2cc9b9;position:relative;flex:none}.tg::after{content:"";position:absolute;top:2px;right:2px;width:18px;height:18px;border-radius:50%;background:#fff;box-shadow:0 1px 3px rgba(0,0,0,.3)}
.tg.off{background:#3a3b42}.tg.off::after{right:auto;left:2px}
.main{display:grid;grid-template-columns:1fr 380px;gap:28px;padding:10px 28px 0 28px;height:574px}
.stage{position:relative;border-radius:14px;background:url(cove-wall.png) center/cover;overflow:hidden}
.stage::before{content:"";position:absolute;inset:0;background:rgba(8,9,11,.25)}
.mbar{position:absolute;top:0;left:0;right:0;height:16px;background:rgba(14,15,18,.5);display:flex;align-items:center;gap:9px;padding:0 9px;font-size:7.5px}
.mbar i{display:block;width:26px;height:4px;border-radius:2px;background:rgba(236,232,227,.6)}
.mbar i.s{width:14px;background:rgba(236,232,227,.35)}
.mdock{position:absolute;bottom:10px;left:50%;transform:translateX(-50%);display:flex;gap:5px;padding:5px 6px;border-radius:12px;background:rgba(22,23,27,.45);border:1px solid rgba(255,255,255,.1)}
.mdock b{width:22px;height:22px;border-radius:6px;display:block}
.sel{position:absolute;bottom:4px;left:50%;transform:translateX(-50%);width:290px;height:44px;border-radius:16px;border:2px solid #2cc9b9;box-shadow:0 0 0 6px rgba(44,201,185,.14)}
.tag{position:absolute;font-size:12px;padding:5px 10px;border-radius:999px;background:rgba(17,18,21,.78);border:1px solid rgba(255,255,255,.1);display:flex;align-items:center;gap:6px}
.tag::before{content:"";width:6px;height:6px;border-radius:50%;background:rgba(236,232,227,.6)}
.tag.on{background:#2cc9b9;color:#062824;border-color:transparent;font-weight:600}.tag.on::before{background:#062824}
.mwin::before,.mwin2::before{content:'';position:absolute;left:0;right:0;top:0;height:20px;border-bottom:1px solid rgba(255,255,255,.06)}
.mwin{position:absolute;left:130px;top:80px;width:300px;height:190px;border-radius:10px;background:rgba(32,33,38,.85);border:1px solid rgba(255,255,255,.08)}
.mwin2{position:absolute;left:330px;top:150px;width:260px;height:170px;border-radius:10px;background:rgba(40,41,47,.9);border:1px solid rgba(255,255,255,.08)}
.side h2{font-size:26px;font-weight:650;letter-spacing:-.02em}
.side p.sub{font-size:13.5px;color:rgba(236,232,227,.58);margin:6px 0 22px;line-height:1.45}
.grp{border-radius:14px;background:#18191d;margin-bottom:14px}
.it{display:flex;align-items:center;justify-content:space-between;gap:14px;padding:13px 16px;font-size:13.5px;border-top:1px solid rgba(255,255,255,.05)}
.it:first-child{border-top:0}
.it small{display:block;color:rgba(236,232,227,.5);font-size:12px;margin-top:2px}
.seg{display:flex;background:#25262c;border-radius:9px;padding:2px;font-size:12.5px}.seg span{padding:5px 11px;border-radius:7px;color:rgba(236,232,227,.7)}.seg span.a{background:#3a3b42;color:#fff}
.rng{width:150px;height:4px;border-radius:2px;background:#33343a;position:relative}.rng b{position:absolute;left:0;top:0;bottom:0;border-radius:2px;background:#2cc9b9}.rng i{position:absolute;top:-7px;width:18px;height:18px;border-radius:50%;background:#fff;box-shadow:0 1px 4px rgba(0,0,0,.4)}
.pins{display:flex;gap:5px}.pins b{width:18px;height:18px;border-radius:5px;display:block}
.scenes{position:absolute;bottom:0;left:0;right:0;height:92px;display:flex;align-items:center;justify-content:center;gap:8px}
.sc{display:flex;align-items:center;gap:8px;padding:9px 16px;border-radius:12px;font-size:13.5px;color:rgba(236,232,227,.62)}
.sc.a{background:#1d1e23;color:#fff}
.sc svg{width:16px;height:16px}
'''
cols=['#1f6feb','#e8c9a8','#0e639c','#1db954','#5865f2','#25d366','#2c2c2c','#f4f1ec','#3b2a63']
mdock='<div class="mdock">'+''.join(f'<b style="background:{c}"></b>' for c in cols)+'</div>'
pins='<div class="pins">'+''.join(f'<b style="background:{c}"></b>' for c in cols[:6])+'</div>'
def scn(lbl,a=False,d=''): return f'<div class="sc{" a" if a else ""}"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round">{d}</svg>{lbl}</div>'
scenes=''.join([scn('Desktop',d='<rect x="3" y="4" width="18" height="13" rx="2"/><path d="M9 21h6M12 17v4"/>'),
 scn('Dock',True,'<rect x="3" y="14" width="18" height="6" rx="3"/><path d="M7 10h.01M12 10h.01M17 10h.01"/>'),
 scn('Menu bar',d='<rect x="3" y="4" width="18" height="4" rx="1.5"/><path d="M3 13h18M3 18h12" opacity=".5"/>'),
 scn('Windows',d='<rect x="3" y="5" width="11" height="9" rx="1.5"/><rect x="10" y="10" width="11" height="9" rx="1.5"/>'),
 scn('Keys',d='<rect x="2.5" y="6" width="19" height="12" rx="2"/><path d="M6 10h.01M10 10h.01M14 10h.01M18 10h.01M8 14h8"/>'),
 scn('Quiet',d='<path d="M20 14.5A8 8 0 1 1 9.5 4a6.5 6.5 0 0 0 10.5 10.5z"/>')])
win=f'''<div class="win"><div class="top"><div class="brand">{P["mark"].replace('class="mk"','')}Cove</div>
<div class="master">Cove desktop<span class="tg"></span></div>
<div class="caps"><span>—</span><span>▢</span><span>✕</span></div></div>
<div class="main"><div class="stage"><div class="mbar"><i></i><i class="s"></i><i class="s"></i><span style="flex:1"></span><i class="s"></i><i></i></div>
<div class="mwin"></div><div class="mwin2"></div>{mdock}<div class="sel"></div>
<span class="tag" style="top:24px;left:16px">Menu bar</span><span class="tag" style="top:46%;left:46%">Windows</span><span class="tag" style="top:22%;right:22px">Desktop</span><span class="tag on" style="bottom:62px;left:50%;transform:translateX(-50%)">Dock</span></div>
<div class="side"><h2>Dock</h2><p class="sub">Your taskbar, rebuilt. Click any part of the desktop on the left to change it.</p>
<div class="grp"><div class="it"><div>Pinned apps<small>Brought over from your taskbar</small></div>{pins}</div>
<div class="it">Position<div class="seg"><span>Left</span><span class="a">Bottom</span><span>Right</span></div></div>
<div class="it">Size<div class="rng"><b style="width:42%"></b><i style="left:36%"></i></div></div>
<div class="it">Magnification<div class="rng"><b style="width:64%"></b><i style="left:58%"></i></div></div></div>
<div class="grp"><div class="it"><div>Hide when a window touches it</div><span class="tg off"></span></div>
<div class="it"><div>Bounce while apps open</div><span class="tg"></span></div>
<div class="it"><div>Folders on the dock<small>Downloads</small></div><span class="muted" style="font-size:13px">Edit</span></div></div></div></div>
<div class="scenes">{scenes}</div></div>'''
page('s3.html',st_css,P['dock'].replace('app hover','app').replace('<span class=lbl>Spotify</span>','')+win)
