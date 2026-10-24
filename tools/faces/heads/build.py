# Build app/ui/screens/heads/heads.js (faces F14): heads.src.js with the skin and eye shaders of the v7 head page
# (the kit's heads_v4.js) put in place, moved to three.js r149 (the app's copy).
# usage: python3 build.py KIT_DIR OUT_JS
import sys

kit, out = sys.argv[1:3]
here = __file__.rsplit('/', 1)[0]
src = open(f'{here}/heads.src.js').read()
v4 = open(f'{kit}/heads_v4.js').read()


def cut(a, b):
    i = v4.index(a); j = v4.index(b, i)
    return v4[i:j].rstrip() + '\n'


skin = cut('const NOISE=`', 'const mesh=new THREE.Mesh(geo,skinMat)')
eyes = cut('const eyeMat=', 'const eyes=')
# r149 splits sheen into a strength and a colour
old = 'sheen:new THREE.Color(0x251510)'
assert old in skin
skin = skin.replace(old, 'sheen:1,sheenColor:new THREE.Color(0x251510),sheenRoughness:.55')
# the approved renders' skin: Blender's specular level .35 (of .5) and no clear coat
skin = skin.replace('clearcoat:.06,clearcoatRoughness:.5,', 'specularIntensity:.55,')
old = 'clamp(brow*mix(.8,1.15,uGnm),0.,1.)'
assert old in skin
skin = skin.replace(old, 'clamp(brow*.8,0.,1.)')
# lips: v7's lip colour (the cheek times the group's measured lip-to-cheek ratio) instead of the v4 tint
old = 'sk=mix(sk,sk*mix(vec3(.95,.55,.55),vec3(.62,.42,.44),uDark)*(1.+.1*uLipT),lips*clamp(.75+.1*uLipT,.5,.9));'
assert old in skin
skin = skin.replace(old, 'sk=mix(sk,sk*uLipR*(1.+.1*uLipT),lips);')
old = 'uniform vec3 uSkin,uHair,uJersey;'
assert old in skin
skin = skin.replace(old, 'uniform vec3 uSkin,uHair,uJersey,uLipR;')
# brows: v7's Blender brows are hair-thin; the v4 page drew them 1.4 times wider for GNM heads
old = 'uBrowA.w*mix(1.,1.4,uGnm)'
assert old in skin
skin = skin.replace(old, 'uBrowA.w')
# the eye whites read too bright under the approved light (the v7 renders' sclera is a dimmer, warmer grey)
old = 'vec3 scl=vec3(.74,.68,.63)'
assert old in eyes
eyes = eyes.replace(old, 'vec3 scl=vec3(.62,.57,.53)')
assert '/*@SKIN@*/' in src and '/*@EYES@*/' in src
js = src.replace('/*@SKIN@*/', skin).replace('/*@EYES@*/', eyes)
head = ('// AFM drawn heads (faces F14). Built by tools/faces/heads/build.py from heads.src.js; edit those, not this file.\n'
        '// The face shapes are Google\'s GNM Head model (Apache 2.0) on MakeHuman\'s base mesh (CC0).\n')
open(out, 'w').write(head + js)
print(out, len(js) // 1024, 'KB')
