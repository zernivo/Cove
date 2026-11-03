const p=require('/workspace/work/intro2/node_modules/puppeteer-core');
(async()=>{const b=await p.launch({executablePath:'/usr/bin/chromium',args:['--no-sandbox','--allow-file-access-from-files']});
const pg=await b.newPage();await pg.setViewport({width:1440,height:900,deviceScaleFactor:1.5});
for(const s of ['s1','s2','s3']){await pg.goto('file://'+__dirname+'/'+s+'.html');await pg.evaluate(()=>document.fonts.ready);await new Promise(r=>setTimeout(r,300));await pg.screenshot({path:s+'.png'});}
await b.close();})();
