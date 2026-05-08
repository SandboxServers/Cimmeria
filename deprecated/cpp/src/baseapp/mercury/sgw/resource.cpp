#include <stdafx.hpp>
#include <baseapp/mercury/sgw/resource.hpp>
#include <boost/python/import.hpp>
#include <entity/pyutil.hpp>

namespace Mercury
{
namespace SGW
{

/*
 * Internal names assigned to each resource category.
 * Python resource handler must use the same names when constructing a resource.
 */
const char * ResourceManager::categoryNames_[ResourceCategories + 1] = {
	"",
	"kismet_event_sequence",
	"ability",
	"mission",
	"item",
	"dialog",
	"kismet_event_set",
	"char_creation",
	"interaction_set_map",
	"effect",
	"text",
	"error_text",
	"world_info",
	"stargate",
	"container",
	"blueprint",
	"applied_science",
	"discipline",
	"racial_paradigm",
	"special_words",
	"interaction",
	"pet_command",
	"behavior_event"
};

/*
 * Retrieves a resource from the resource manager.
 * If the resource is cached, it is returned from cache; otherwise the underlying
 * game code (base.createResource) is called to create a cooked XML from the resource.
 * 
 * @param categoryId Resource category ID (see ResourceManager::categoryNames)
 * @param elementId  Resource ID (eg. ability ID, item ID, ...)
 *
 * @return Cooked resource XML or nullptr if the resource request failed
 */
std::string const * ResourceManager::get(uint32_t categoryId, uint32_t elementId)
{
	std::string const * res = getFromCache(categoryId, elementId);
	if (res)
		return res;
	
	/*
	 * No resource was found, try to create one by calling
	 * base.createResource(categoryId, elementId)
	 */
	PyGilGuard guard;
	try
	{
		auto baseMod = bp::import("base");
		auto resource = baseMod.attr("createResource")(bp::object(categoryId), bp::object(elementId));
		PyObject * raw = resource.ptr();
		if (resource && raw && PyBytes_Check(raw))
		{
			resources_.insert(std::make_pair(std::make_pair(categoryId, elementId), PyBytes_AS_STRING(raw)));
			res = getFromCache(categoryId, elementId);
			SGW_ASSERT(res);
		}
		else
		{
			WARN("base.createResource() returned illegal resource: categoryId=%d, elementId=%d",
				categoryId, elementId);
		}
	}
	catch (bp::error_already_set &)
	{
		FAULT("Exception while creating resource: categoryId=%d, elementId=%d",
			categoryId, elementId);
		PyUtil_ShowErr();
	}

	return res;
}


/*
 * Retrieves a resource from the internal resource cache.
 * If the resource is not cached, nullptr is returned.
 * 
 * @param categoryId Resource category ID (see ResourceManager::categoryNames)
 * @param elementId  Resource ID (eg. ability ID, item ID, ...)
 *
 * @return Cooked resource XML or nullptr if the resource request failed
 */
std::string const * ResourceManager::getFromCache(uint32_t categoryId, uint32_t elementId) const
{
	auto it = resources_.find(std::make_pair(categoryId, elementId));
	if (it != resources_.end())
		return &it->second;
	else
		return nullptr;
}

}
}
