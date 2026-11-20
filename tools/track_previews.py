"""Bake transparent track-picker illustrations at sixteen camera-relative headings."""
from math import cos, sin, pi, tau
from pathlib import Path
from PIL import Image, ImageDraw

W, H, S = 128, 96, 3
atlas = Image.new("RGBA", (W * 16, H * 8))

def thumbnail(kind, angle):
    faces = []
    def face(points, color, layer=0):
        # Back to front in an elevated orthographic view.
        depth = sum(z + y * .76 for x, y, z in points) / len(points)
        faces.append((layer, depth, [(x, z * .60 - y * .80) for x, y, z in points], color))
    turn = {1: -pi/2, 2: pi/2}.get(kind, 0)
    rise = {3: -2.2, 4: 2.2}.get(kind, 0)
    def pos(t, side, h):
        a = angle + turn*t
        if turn:
            x = 4*(sin(a)-sin(angle)) * (1 if turn > 0 else -1)
            z = 4*(cos(angle)-cos(a)) * (1 if turn > 0 else -1)
        else:
            x, z = 6*t*cos(angle), 6*t*sin(angle)
        return (x-side*sin(a), max(-rise, 0)+rise*t+h, z+side*cos(a))
    base, wall, rim = (("#f19a41","#a85624","#ffc473") if kind < 4 else
                       ("#448874","#244e46","#97cbb0") if kind == 4 else
                       ("#d85b41","#8f342d","#f29165"))
    def strip(a,b,l,r,h,color,layer=0):
        face([pos(a,l,h),pos(b,l,h),pos(b,r,h),pos(a,r,h)],color,layer)
    def roller(t,side):
        ring=[(t+.052*cos(i*tau/16),side+.31*sin(i*tau/16)) for i in range(16)]
        for i,(p,q) in enumerate(zip(ring,ring[1:]+ring[:1])):
            face([pos(*p,.14),pos(*q,.14),pos(*q,.59),pos(*p,.59)],"#383d3c",1)
        face([pos(*p,.59) for p in ring],"#565b53",2)
    if kind == 7:
        for sign in (-1, 1):
            turn = sign * pi / 2
            for j in range(24):
                a, b = j / 24, (j + 1) / 24
                strip(a, b, -1, 1, 0, "#438c73")
                for side in (-1, 1):
                    if side == sign or a > .44:
                        face([pos(a,side,0),pos(b,side,0),pos(b,side,.48),pos(a,side,.48)], "#26574b")
                        strip(a,b,side,.84*side,.48,"#9ad5b3")
            t=.76
            face([pos(t,-.58,.05),pos(t+.10,0,.05),pos(t,.58,.05),pos(t+.065,.58,.05),
                  pos(t+.165,0,.05),pos(t+.065,-.58,.05)],"#fff2ce",1)
        turn = 0
        face([pos(.04,-.85,.5),pos(.06,-.56,.5),pos(.63,.48,.5),pos(.64,.18,.5)],"#d79f4e",2)
        face([pos(.47,-.12,.65),pos(.47,.12,.65),pos(.65,.96,.95),pos(.69,.69,.95)],"#e9ba67",3)
        ring=[pos(.67+.08*cos(i*tau/16),.84+.42*sin(i*tau/16),1.0) for i in range(16)]
        face(ring,"#dc593b",4)
    elif kind==6:
        ring=[(2.2*cos(i*tau/40),2.2*sin(i*tau/40)) for i in range(40)]
        for p,q in zip(ring,ring[1:]+ring[:1]):
            face([(p[0]*.86,0,p[1]*.86),(q[0]*.86,0,q[1]*.86),(q[0],2,q[1]),(p[0],2,p[1])],"#a7d1ce")
        face([(x,2,z) for x,z in ring],"#e8f4ed",1)
        face([(x*.89,2,z*.89) for x,z in ring],"#63b7bf",2)
    else:
        for j in range(24):
            a,b=j/24,(j+1)/24
            strip(a,b,-1,1,0,base)
            for side in (-1,1):
                face([pos(a,side,-.18),pos(b,side,-.18),pos(b,side,.38),pos(a,side,.38)],wall)
                strip(a,b,side,side*.86,.38,rim)
                face([pos(a,side*.86,0),pos(b,side*.86,0),pos(b,side*.86,.38),pos(a,side*.86,.38)],wall)
        if kind in (3,4):
            for t in (.07,.92):
                for side in (-.73,.73):
                    p=pos(t,side,-.15)
                    face([(p[0]-.15,-.32,p[2]),(p[0]+.15,-.32,p[2]),(p[0]+.15,p[1],p[2]),(p[0]-.15,p[1],p[2])],"#bc9259",-1)
        if kind==4:
            for side in (-.79,.79):
                strip(.025,.975,side-.06,side+.06,.18,"#344140",1)
                strip(.025,.975,side-.06,side+.06,-.55,"#344140",-1)
                for t in (.035,.965):
                    face([pos(t+.057*cos(i*tau/24),side,-.19+.35*sin(i*tau/24)) for i in range(24)],"#c49655",1)
            for t in (.12,.55,.94):
                strip(t,t+.036,-.74,.74,.38,"#3d514d",2)
        if kind==5:
            for side in (-.75,.75): roller(.45,side)
        # A raised ivory chevron indicates travel, including uphill and around bends.
        for t in (.28,.68):
            face([pos(t,-.54,.12),pos(t+.095,0,.12),pos(t,.54,.12),pos(t+.05,.54,.12),pos(t+.148,0,.12),pos(t+.05,-.54,.12)],"#fff2ce",2)
    points=[p for _,_,poly,_ in faces for p in poly]
    xs,ys=zip(*points)
    k=min((W-12)/(max(xs)-min(xs)),(H-12)/(max(ys)-min(ys)),19)
    cx,cy=(max(xs)+min(xs))/2,(max(ys)+min(ys))/2
    im=Image.new("RGBA",(W*S,H*S))
    draw=ImageDraw.Draw(im)
    for _,_,poly,col in sorted(faces,key=lambda f:f[:2]):
        draw.polygon([((W/2+(x-cx)*k)*S,(H/2+(y-cy)*k)*S) for x,y in poly],fill=col)
    return im.resize((W,H),Image.Resampling.LANCZOS)

for kind in range(8):
    for view in range(1 if kind==6 else 16):
        atlas.paste(thumbnail(kind,view*tau/16),(view*W,kind*H))
atlas.save(Path(__file__).resolve().parents[1]/"assets"/"track-picker.png",optimize=True)
