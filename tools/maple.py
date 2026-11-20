from PIL import Image
import sys
im=Image.open(sys.argv[1]).convert('RGB')
for i,size in enumerate([64,96,128,192,256,384,512,640,800,1024]):
    im.resize((size,size),Image.Resampling.LANCZOS).save(f'assets/maple-{i}.jpg',quality=83+i)
