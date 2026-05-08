#pragma once

namespace Mercury
{
namespace SGW
{

/*
 * Responsible for requesting cooked XML resources from Python scripts and caching generated resources.
 */
class ResourceManager
{
	public:
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
		std::string const * get(uint32_t categoryId, uint32_t elementId);

		/*
		 * Retrieves a resource from the internal resource cache.
		 * If the resource is not cached, nullptr is returned.
		 * 
		 * @param categoryId Resource category ID (see ResourceManager::categoryNames)
		 * @param elementId  Resource ID (eg. ability ID, item ID, ...)
		 *
		 * @return Cooked resource XML or nullptr if the resource request failed
		 */
		std::string const * getFromCache(uint32_t categoryId, uint32_t elementId) const;

	private:
		static const unsigned int ResourceCategories = 22;
		static const char * categoryNames_[ResourceCategories + 1];

		// List of cached resources - (categoryId, elementId, resourceXml) tuples
		std::map<std::pair<uint32_t, uint32_t>, std::string> resources_;
};


}
}
