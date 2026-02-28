import Atrea
from xml.sax.saxutils import escape
from common.defs.Resource import Resource


class Discipline(Resource):
	versioning = True

	def __init__(self, row, defMgr):
		self.id = row['discipline_id']
		self.name = row['name']
		self.icon = row['icon']
		self.appliedScience = defMgr.require('applied_science', row['applied_science_id'])
		self.racialParadigm = defMgr.require('racial_paradigm', row['racial_paradigm_id'])
		self.racialParadigmLevel = row['racial_paradigm_level']
		self.techCompetency = row['tech_competency']
		self.row = row['row']
		self.column = row['column']
		self.requiredDisciplines = [int(e) for e in row['required_discipline_ids'][1:-1].split(',') if e]

	def postLoad(self, defMgr):
		# Discipline dependencies must be postloaded because a discipline can reference ones that are loaded later
		self.requiredDisciplines = [defMgr.require('discipline', e) for e in self.requiredDisciplines]

	def toXml(self):
		""" Creates a cooked XML resource from the discipline definition """
		xml = '<?xml version="1.0" encoding="UTF-8"?><COOKED_DISCIPLINE>'
		xml += '<ID>' + str(self.id) + '</ID>'
		xml += '<Name>' + escape(self.name) + '</Name>'
		xml += '<Icon>' + self.icon + '</Icon>'
		xml += '<AppliedScienceID>' + str(self.appliedScience.id) + '</AppliedScienceID>'
		xml += '<RacialParadigmID>' + str(self.racialParadigm.id) + '</RacialParadigmID>'
		xml += '<RacialParadigmLevel>' + str(self.racialParadigmLevel) + '</RacialParadigmLevel>'
		xml += '<TechCompetency>' + str(self.techCompetency) + '</TechCompetency>'
		xml += '<Row>' + str(self.row) + '</Row>'
		xml += '<Column>' + str(self.column) + '</Column>'
		for discipline in self.requiredDisciplines:
			xml += '<RequiredDisciplineIDs>' + str(discipline.id) + '</RequiredDisciplineIDs>'
		xml += '</COOKED_DISCIPLINE>'
		return xml

	@staticmethod
	def loadAll(defs, defMgr):
		disciplines = Atrea.dbQuery('select * from resources.disciplines')
		for discipline in disciplines:
			defs[discipline['discipline_id']] = Discipline(discipline, defMgr)
