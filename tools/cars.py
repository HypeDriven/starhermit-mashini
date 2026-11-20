"""Bake separate die-cast components in an isolated Blender scene."""
import bpy, math, struct, os
from mathutils import Vector
from math import pi, sin, cos, sqrt
root=os.path.join(os.path.dirname(__file__),'..','assets')
original=bpy.context.window.scene
studio=bpy.data.scenes.new('Mashini tooling')
bpy.context.window.scene=studio
def part(mesh,group,mat):
    obj=bpy.data.objects.new('Mashini part',mesh);studio.collection.objects.link(obj)
    parts.append((obj,group,mat));return obj
def bevel(obj,r):
    mod=obj.modifiers.new('Rolled edges','BEVEL');mod.width=r;mod.segments=detail;mod.affect='EDGES'
    obj.modifiers.new('Face normals','WEIGHTED_NORMAL')
def mesh(name,verts,faces,group,mat,smooth=False):
    geo=bpy.data.meshes.new(name);geo.from_pydata(verts,[],faces);geo.update()
    for p in geo.polygons:p.use_smooth=smooth
    return part(geo,group,mat)
def box(loc,dims,group,mat,r=.02,rot=0):
    bpy.ops.mesh.primitive_cube_add(size=1,location=loc);o=bpy.context.object
    for v in o.data.vertices:
        for j in range(3):v.co[j]*=dims[j]
    o.rotation_euler.y=rot;parts.append((o,group,mat))
    if r:bevel(o,r)
    return o
def cyl(loc,rad,depth,group,mat,ax='Y',r=.012):
    bpy.ops.mesh.primitive_cylinder_add(vertices=8+detail*8,radius=rad,depth=depth,location=loc)
    o=bpy.context.object;o.rotation_euler.x=pi/2 if ax=='Y' else 0
    if ax=='X':o.rotation_euler.y=pi/2
    parts.append((o,group,mat));bevel(o,r);return o
def loft(rows,group,mat,cabin=None):
    vs=[]
    for x,w,b,t in rows:
        vs.extend([(x,-w*.84,b),(x,-w*.97,b+.035),(x,-w,max(b+.045,t-.073)),(x,-w*.83,t),(x,w*.83,t),(x,w,max(b+.045,t-.073)),(x,w*.97,b+.035),(x,w*.84,b)])
    faces=[tuple(range(8))]+[tuple(reversed(range(len(vs)-8,len(vs))))]
    for i in range(len(rows)-1):
        for j in range(8):
            if cabin and j==3 and cabin[0]<rows[i][0]<cabin[1]:continue
            faces.append((8*i+j,8*(i+1)+j,8*(i+1)+(j+1)%8,8*i+(j+1)%8))
    o=mesh('Die forming',vs,faces,group,mat);bevel(o,.025)
def make(kind):
    top=[.59,.68,.63][kind];length=[1.32,1.38,1.32][kind]
    rows=[]
    for i in range(61):
        x=-length+2*length*i/60;rounded=max(0,(abs(x)/length-.80)/.20)
        arch=max(0,.32**2-(abs(x)-.81)**2)
        bottom=.235+sqrt(arch) if arch else .235
        rows.append((x,.51-.12*rounded**2,bottom,top+.05*math.exp(-((abs(x)-.81)/.38)**2)-.055*rounded))
    back,front=[(-.84,.51),(-1.16,.88),(-.25,.80)][kind];roof=top+[.42,.57,.51][kind]
    loft(rows,1,0,(back,front));box(((front+back)/2,0,.39),(front-back,.85,.055),3,1)
    # Individually formed window panels, piers, roof, and rain rails.
    for side in [-1,1]:
        y=side*.447;ry=side*.36
        mesh('Side glazing',[(back,y,top),(front,y,top),(front-.29,ry,roof),(back+.2,ry,roof)],[(0,1,2,3)],2,3)
        for frac in [0,.50,1]:
            a=Vector((back+(front-back)*frac,y,top));b=Vector((back+.2+(front-back-.49)*frac,ry,roof))
            o=box((a+b)/2,(.039,.039,(b-a).length),1,0,.01);o.rotation_mode='QUATERNION';o.rotation_quaternion=(b-a).to_track_quat('Z','Y')
    for x,up in [(front,front-.29),(back,back+.2)]:
        mesh('Screen',[(x,-.447,top),(x,.447,top),(up,.36,roof),(up,-.36,roof)],[(0,1,2,3)],2,3)
    roofstart=back+.16;roofend=front-.25
    sections=[]
    for i in range(15):
        u=i/14;edge=abs(2*u-1)**3
        sections.append((roofstart+(roofend-roofstart)*u,.397-.074*edge,roof-.021,roof+.082-.029*edge))
    loft(sections,1,0)

    box((0,0,.205),(2.42,.82,.12),0,1,.05)
    for x in [-.81,.81]:cyl((x,0,.27),.057,1.10,0,4)
    for seat in range(2 if kind==1 else 1):
        for y in [-.19,.19]:
            x=(front+back)/2-seat*.52
            box((x,y,.49),(.38,.31,.12),3,5,.05)
            box((x-.17,y,.65),(.12,.31,.33),3,5,.05,-.13)
    box((front-.15,0,.65),(.23,.76,.1),3,1,.035)
    bpy.ops.mesh.primitive_torus_add(major_segments=16+detail*6,minor_segments=6,location=(front-.29,-.20,.73),major_radius=.10,minor_radius=.013)
    steering=bpy.context.object;steering.rotation_euler.y=1.17;parts.append((steering,3,1))
    box((front-.29,-.20,.73),(.023,.18,.017),3,4,.006)
    box((front-.29,-.20,.686),(.023,.02,.09),3,4,.006)
    box((.1,0,.51),(.43,.09,.15),3,1,.035)
    for yy in [-.22,-.13]:cyl((front-.265,yy,.65),.036,.012,3,4,'X')
    # Seat stitching, head restraints and floor mats are separate insert details.
    for yy in [-.19,.19]:
        xx=(front+back)/2
        box((xx-.17,yy,.82),(.085,.20,.11),3,5,.032)
        for j in [-1,0,1]:box((xx-.103,yy+j*.071,.65),(.018,.009,.19),3,6,.004,-.13)
    if kind==0:
        for yy in [-.12,.12]:
            stripe=[]
            for x,w,b,t in rows:
                if x>front+.012 or x<back-.03:
                    stripe.append((x,yy,t+.004))
                elif stripe:
                    vs=[(a,y+d,z) for a,y,z in stripe for d in [-.039,.039]]
                    mesh('Cream coach stripe',vs,[(j,j+1,j+3,j+2) for j in range(0,len(vs)-2,2)],1,6);stripe=[]
            if stripe:
                vs=[(a,y+d,z) for a,y,z in stripe for d in [-.039,.039]]
                mesh('Cream coach stripe',vs,[(j,j+1,j+3,j+2) for j in range(0,len(vs)-2,2)],1,6)
            box(((front+back-.09)/2,yy,roof+.083),(front-back-.47,.078,.006),1,6,.002)
    # Front grille, bonnet vent and badges.
    box((length+.009,0,top-.15),(.035,.46,.124),1,1,.021)
    for j in range(5):box((length+.030,0,top-.19+j*.021),(.008,.42,.007),1,4,.002)
    box((length-.15,0,top-.002),(.07,.052,.012),1,4,.006)
    if kind==1:
        for yy in [-.29,.29]:box(((front+back)/2,yy,roof+.135),(1.17,.026,.036),1,4)
        for xx in [-.53,.26]:box((xx,0,roof+.142),(.036,.65,.025),1,4)
    if kind==2:
        for yy in [-.43,.43]:box((-.76,yy,top+.075),(.95,.075,.18),1,0,.031)
        for yy in [-.3,-.15,0,.15,.3]:box((-.78,yy,top+.033),(.80,.014,.025),1,4,.005)
    for sign in [-1,1]:
        box((front-.09,sign*.54,top+.04),(.11,.12,.03),1,4,.01)
        box((front-.10,sign*.61,top+.07),(.13,.075,.09),1,0,.03)
        if kind==0:
            cyl((.04,sign*.513,.47),.108,.015,1,6)
            box((.04,sign*.525,.47),(.022,.012,.125),1,1,.004,-.19)

    if kind==2:box((-.78,0,top+.008),(.86,.78,.025),1,1)
    for side in [-1,1]:
        box((0,side*.505,.34),(1.0,.025,.035),1,4)
        box((front-.39,side*.515,top-.06),(.12,.025,.032),1,4)
        for x,mat in [(length,7),(-length,8)]:
            cyl((x,side*.325,top-.11),.09,.043,1,4,'X');cyl((x+(.027 if x>0 else -.027),side*.325,top-.11),.071,.028,1,mat,'X')
    for x in [-length,length]:box((x,0,.30),(.10,.89,.077),1,4,.035)
    for idx,(x,y) in enumerate([(x,y) for x in [-.81,.81] for y in [-.51,.51]]):
        group=4+idx;side=1 if y>0 else -1
        cyl((x,y,.27),.269,.19,group,1,r=.044)
        if detail>=3:
            for rad in [.216,.235]:
                bpy.ops.mesh.primitive_torus_add(major_segments=32+detail*6,minor_segments=5,location=(x,y+side*.094,.27),major_radius=rad,minor_radius=.004)
                lip=bpy.context.object;lip.rotation_euler.x=pi/2;parts.append((lip,group,1))
            for j in range(24):
                a=j*pi/12
                o=box((x+.267*cos(a),y,.27+.267*sin(a)),(.006,.097,.017),group,1,0)
                o.rotation_euler.y=-a
        cyl((x,y+.091*side,.27),.188,.023,group,6)
        cyl((x,y+.106*side,.27),.142,.033,group,4)
        cyl((x,y+.128*side,.27),.063,.034,group,0)
        if detail>=2:
            for j in range(6):
                a=j*pi/3;cyl((x+.097*cos(a),y+.126*side,.27+.097*sin(a)),.021,.009,group,1)
def bake(kind,lod):
    bpy.context.view_layer.update();dg=bpy.context.evaluated_depsgraph_get();batches={}
    for o,group,mat in parts:
        eo=o.evaluated_get(dg);geo=eo.to_mesh();geo.calc_loop_triangles();m=eo.matrix_world
        verts,faces,cache=batches.setdefault((group,mat),([],[],{}))
        for tri in geo.loop_triangles:
            for vi,li in zip(tri.vertices,tri.loops):
                v=m@geo.vertices[vi].co;n=m.to_3x3()@geo.corner_normals[li].vector
                key=tuple(round(k,5) for k in (v.x,v.z,-v.y,n.x,n.z,-n.y))
                if key not in cache:cache[key]=len(verts);verts.append(key)
                faces.append(cache[key])
        eo.to_mesh_clear()
    with open(os.path.join(root,f'car-{kind}-{lod}.mesh'),'wb') as f:
        f.write(struct.pack('<I',len(batches)))
        for (g,m),(vs,ix,_) in sorted(batches.items()):
            f.write(struct.pack('<4I',g,m,len(vs),len(ix)))
            for v in vs:f.write(struct.pack('<6f',*v))
            f.write(struct.pack('<'+'H'*len(ix),*ix))
try:
    for lod,detail in enumerate([1,2,3,5]):
        for kind in range(3):
            parts=[];make(kind);bake(kind,lod)
            for o,_,_ in parts:
                geo=o.data;bpy.data.objects.remove(o,do_unlink=True);bpy.data.meshes.remove(geo)
finally:
    bpy.context.window.scene=original;bpy.data.scenes.remove(studio)
