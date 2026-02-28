from os import listdir
from os.path import isfile, join


class Reader(object):
	def __init__(self, text):
		self.text = text
		self.position = 0


	def readWhitespace(self):
		while self.text[self.position] == ' ' or self.text[self.position] == '\r' or self.text[self.position] == '\n':
			self.position += 1


	def readWord(self):
		self.readWhitespace()
		start = self.position
		while True:
			c = ord(self.text[self.position].lower())
			if ord('a') <= c <= ord('z') or ord('0') <= c <= ord('9') or c == ord('_') or c == ord('-'):
				self.position += 1
			else:
				break
		return self.text[start:self.position]


	def readString(self):
		q = self.text[self.position]
		assert q == '"' or q == "'"
		self.position += 1
		start = self.position
		escaped = False
		while True:
			c = self.text[self.position]
			if c == q and not escaped:
				break
			elif c == '\\' and not escaped:
				escaped = True
			else:
				escaped = False
			self.position += 1
		str = self.text[start:self.position]
		self.position += 1
		return str


	def readNumber(self):
		start = self.position
		while ord('0') <= ord(self.text[self.position]) <= ord('9') or self.text[self.position] == '-' or self.text[self.position] == '.':
			self.position += 1
		return float(self.text[start:self.position])


	def readKey(self):
		key = self.readWord()
		if self.text[self.position] == '(':
			self.position += 1
			index = self.readNumber()
			assert self.text[self.position] == ')'
			self.position += 1
			return key, int(index)
		else:
			return key, None


	def readArray(self):
		array = {}
		assert self.text[self.position] == '('
		self.position += 1
		index = 0
		while self.text[self.position] != ')':
			kstart = self.position
			key = self.readWord()
			# print(key)
			if key == '' or self.text[self.position] != '=':
				self.position = kstart
				self.readWhitespace()
				value = self.readAnyType()
				array[index] = value
				index += 1
			else:
				self.position += 1
				value = self.readAnyType()
				array[key] = value
			if self.text[self.position] == ',':
				self.position += 1
			else:
				assert self.text[self.position] == ')'
		self.position += 1
		return array


	def readAnyType(self):
		c = self.text[self.position]
		if c == '(':
			return self.readArray()
		elif c == '"' or c == "'":
			return self.readString()
		elif ord('0') <= ord(c) <= ord('9') or c == '-':
			return self.readNumber()
		else:
			word = self.readWord()
			c = self.text[self.position]
			if c == "'":
				name = self.readString()
				return {'name': name, 'type': word}
			else:
				return word


	def readProperty(self):
		self.readWhitespace()
		key = self.readKey()
		# print("Read key: %s(%s)" % key)
		assert self.text[self.position] == '='
		self.position += 1
		value = self.readAnyType()
		# print("Read value: %s" % value)
		return key, value


	def readObject(self):
		word = self.readWord()
		# print(word)
		assert word == "Begin"
		word = self.readWord()
		# print(word)
		assert word == "Object"
		obj = {}
		while True:
			pos = self.position
			word = self.readWord()
			if word == "End":
				break
			self.position = pos
			if word == "Begin":
				readObj = self.readObject()
				obj[readObj['Name']] = readObj
			else:
				key, value = self.readProperty()
				k, index = key
				if index is None:
					obj[k] = value
				elif k in obj:
					obj[k][index] = value
				else:
					obj[k] = {index: value}
		word = self.readWord()
		assert word == "Object"
		return obj


def processObject(object, package, exports):
	if object['Class'] == 'BodyComponent':
		component = {
			'Package': package,
			'Name': object['Name'],
			'BodySets': [],
			'BodySlots': [],
			'Meshes': []
		}
		if 'BodyVisuals' in object:
			for index, visual in object['BodyVisuals'].items():
				bodySets = [bs['name'] for bs in visual['BodySetList'].values() if bs != 'None'] if 'BodySetList' in visual else []
				vset = {
					'Index': index,
					'Component': package + '.' + component['Name'],
					'BodySets': bodySets,
					'Meshes': [],
					'Slots': []
				}
				exports['visualSets'][package + '.' + component['Name'] + '.' + str(index)] = vset
				for bodySet in bodySets:
					if bodySet not in component['BodySets']:
						component['BodySets'].append(bodySet)
				if 'CompositeMeshes' in visual:
					for mesh in visual['CompositeMeshes'].values():
						if 'SkelMesh' in mesh and mesh['SkelMesh']['name'] not in component['Meshes']:
							component['Meshes'].append(mesh['SkelMesh']['name'])
							if 'BodyUVs' in mesh:
								for uv in mesh['BodyUVs'].values():
									if uv['BodySlot'] not in component['BodySlots']:
										component['BodySlots'].append(uv['BodySlot'])
						if mesh not in vset['Meshes']:
							if 'SkelMesh' in mesh:
								vset['Meshes'].append(mesh['SkelMesh']['name'])
								if 'BodyUVs' in mesh:
									for uv in mesh['BodyUVs'].values():
										if uv['BodySlot'] not in vset['Slots']:
											vset['Slots'].append(uv['BodySlot'])

		exports['components'][package + '.' + component['Name']] = component
	elif object['Class'] == 'BodySet':
		bodySet = {
			'Package': package,
			'Name': object['Name'],
			'RefSkelMesh': object['RefSkelMesh']['name'] if 'RefSkelMesh' in object else ''
		}
		exports['bodySets'][package + '.' + bodySet['Name']] = bodySet
	elif object['Class'] == 'StaticMesh':
		mesh = {
			'Package': package,
			'Name': object['Name']
		}
		exports['staticMeshes'][package + '.' + mesh['Name']] = mesh
	elif object['Class'] == 'SkeletalMesh':
		skelMesh = {
			'Package': package,
			'Name': object['Name'],
			'Sockets': [],
			'Bones': []
		}
		for name, socket in object.items():
			if name[0:19] == 'SkeletalMeshSocket_':
				if socket['SocketName'] not in skelMesh['Sockets']:
					skelMesh['Sockets'].append(socket['SocketName'])
				if socket['BoneName'] not in skelMesh['Bones']:
					skelMesh['Bones'].append(socket['BoneName'])
		exports['skeletalMeshes'][package + '.' + skelMesh['Name']] = skelMesh
	elif object['Class'] == 'ComponentKismetData':
		pass
	else:
		raise Exception('Invalid object class: ' + object['Class'])



def processDir(upath, path, dat):
	for object in listdir(path):
		objpath = join(path, object)
		if isfile(objpath):
			f = open(objpath)
			s = f.read()
			f.close()
			r = Reader(s)
			print('Processing %s' % objpath)
			obj = r.readObject()
			processObject(obj, upath, dat)
		else:
			processDir(upath + '.' + object if upath else object, objpath, dat)

exports = {
	'staticMeshes': {},
	'skeletalMeshes': {},
	'components': {},
	'bodySets': {},
	'visualSets': {}
}
processDir('', '../../data/visuals', exports)

f = open('meshes.csv', 'w', encoding='UTF-8')
for mesh in exports['staticMeshes'].values():
	f.write(mesh['Package'] + '.' + mesh['Name'] + '\n')
f.close()

f = open('body_sets.csv', 'w', encoding='UTF-8')
for mesh in exports['bodySets'].values():
	f.write(mesh['Package'] + '.' + mesh['Name'] + '\t' + mesh['RefSkelMesh'] + '\n')
f.close()

f = open('skel_meshes.csv', 'w', encoding='UTF-8')
for mesh in exports['skeletalMeshes'].values():
	f.write(mesh['Package'] + '.' + mesh['Name'] + '\t')
	sockets = '{' + ','.join([s for s in mesh['Sockets']]) + '}'
	f.write(sockets + '\t')
	bones = '{' + ','.join([s for s in mesh['Bones']]) + '}'
	f.write(bones + '\n')
f.close()

f = open('components.csv', 'w', encoding='UTF-8')
for mesh in exports['components'].values():
	f.write(mesh['Package'] + '.' + mesh['Name'] + '\t')
	sets = '{' + ','.join([s for s in mesh['BodySets']]) + '}'
	f.write(sets + '\t')
	slots = '{' + ','.join([s for s in mesh['BodySlots']]) + '}'
	f.write(slots + '\t')
	meshes = '{' + ','.join([s for s in mesh['Meshes']]) + '}'
	f.write(meshes + '\n')
f.close()

f = open('visual_sets.csv', 'w', encoding='UTF-8')
for mesh in exports['visualSets'].values():
	f.write(mesh['Component'] + '\t' + str(mesh['Index']) + '\t')
	sets = '{' + ','.join([s for s in mesh['BodySets']]) + '}'
	f.write(sets + '\t')
	slots = '{' + ','.join([s for s in mesh['Slots']]) + '}'
	f.write(slots + '\t')
	meshes = '{' + ','.join([s for s in mesh['Meshes']]) + '}'
	f.write(meshes + '\n')
f.close()
