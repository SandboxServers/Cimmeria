import Atrea
from common.defs.Resource import Resource


class ItemListItem(object):
	"""
	@ivar id: Unique identifier of item list entry
	@ivar design: Design of this item
	@ivar quantity: Amount of items available
	@ivar naquadahCost: Cost in naquadah
	@ivar itemCosts: List of item costs - (design, quantity) tuples
	"""
	def __init__(self, row, defMgr):
		self.id = row['item_id']
		self.design = defMgr.require('item', row['design_id'])
		self.quantity = row['quantity']
		self.naquadahCost = row['naquadah']
		self.itemCosts = []


	def addCost(self, row, defMgr):
		self.itemCosts.append((defMgr.require('item', row['design_id']), row['quantity']))


class ItemList(Resource):
	"""
	@ivar id: Unique identifier of item list
	@ivar name: Internal name of list
	@ivar items: Items in this list
	"""
	def __init__(self, row, defMgr):
		self.id = row['item_list_id']
		self.name = row['name']
		self.items = {}
		self.indexedItems = []


	@staticmethod
	def loadAll(defs, defMgr):
		lists = Atrea.dbQuery('select * from resources.item_lists')
		for i in lists:
			defs[i['item_list_id']] = ItemList(i, defMgr)

		items = Atrea.dbQuery('select * from resources.item_list_items')
		listItems = {}
		for i in items:
			item = ItemListItem(i, defMgr)
			listItems[item.id] = item
			defs[i['item_list_id']].items[item.id] = item
			defs[i['item_list_id']].indexedItems.append(item)

		prices = Atrea.dbQuery('select * from resources.item_list_prices')
		for p in prices:
			listItems[p['item_id']].addCost(p, defMgr)
