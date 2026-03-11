#pragma once

#include <type_traits>
#include <algorithm>

#if defined(_DEBUG)
#define DEBUG_WORLD_GRID
#endif

template <typename _T>
class WorldGrid;

template <typename _T>
class WorldGridMember
{
public:
	WorldGridMember()
		: grid_(nullptr)
	{}

	~WorldGridMember()
	{
		SGW_ASSERT(!grid_);
	}

protected:
	// Objects that are out of range but can still see us
	std::set<std::weak_ptr<_T>> witnesses_;
	// Objects that are out of range but are still visible to us
	std::set<std::weak_ptr<_T>> visionExceptions_;
	// Grid that we're a member of
	WorldGrid<_T> * grid_;

	friend class WorldGrid<_T>;
};

// Position of an object on the grid
struct WorldGridPosition
{
	inline WorldGridPosition(int x_, int y_)
		: x(x_), y(y_)
	{}

	int x, y;
};

template <typename _T>
class WorldGrid
{
public:
	WorldGrid(WorldGrid const &) = delete;
	WorldGrid & operator=(WorldGrid const &) = delete;

	static_assert(std::is_base_of<WorldGridMember<_T>, _T>::value, "Objects must be subclasses of WorldGridMember");

	struct Params
	{
		// Width and height of the world
		int width, height;
		// Minimal values of X and Y coords
		int minX, minY;
		// X and Y size of each chunk
		int sizeX, sizeY;
		// Max vision distance of entities (chunks)
		int visionDistance;
		// Vision hysteresis in addition to vision distance (chunks)
		int hysteresis;
	};

	struct Chunk
	{
		// List of objects currently on this chunk
		std::set<typename _T::WeakPtr> objects;
		// List of objects that have visibility controllers
		std::set<typename _T::WeakPtr> witnesses;

		void insert(typename _T::WeakPtr object)
		{
			SGW_ASSERT(objects.find(object) == objects.end());
			objects.insert(object);
			
			if (object.lock()->isAWitness())
			{
				SGW_ASSERT(witnesses.find(object) == witnesses.end());
				witnesses.insert(object);
			}
		}

		void remove(typename _T::WeakPtr object)
		{
			auto it = objects.find(object);
			SGW_ASSERT(it != objects.end());
			objects.erase(it);

			if (object.lock()->isAWitness())
			{
				auto witnessIt = witnesses.find(object);
				SGW_ASSERT(witnessIt != witnesses.end());
				witnesses.erase(witnessIt);
			}
		}
	};

	WorldGrid(Params const & params)
		: params_(params)
	{
		SGW_ASSERT(params_.width >= 0 && params_.height >= 0);
		SGW_ASSERT(params_.sizeX >= 0 && params_.sizeY >= 0);
		SGW_ASSERT(params_.visionDistance >= 0);
		SGW_ASSERT(params_.hysteresis >= 0);

		chunksX_ = std::max((int)(params_.width / params_.sizeX + ((params_.width % params_.sizeX) ? 1 : 0)), (int)1);
		chunksY_ = std::max((int)(params_.height / params_.sizeY + ((params_.height % params_.sizeY) ? 1 : 0)), (int)1);
		chunks_.resize(chunksX_ * chunksY_);
	}

	~WorldGrid()
	{
		for (auto const & chunk : chunks_)
		{
			for (auto const & weakObject : chunk.objects)
			{
				auto object = weakObject.lock();
				object->witnesses_.clear();
				object->visionExceptions_.clear();
				object->grid_ = nullptr;
			}
		}
	}

	
	/*
	 * Checks if the specified world position is valid.
	 *
	 * @param pos Position to check
	 */
	bool validPosition(WorldGridPosition const & pos) const
	{
		return pos.x >= params_.minX && pos.x < params_.minX + params_.width &&
		       pos.y >= params_.minY && pos.y < params_.minY + params_.height;
	}


	/*
	 * Returns which chunk the specified *world* position belongs to.
	 *
	 * @param pos Position to check
	 */
	unsigned int chunkIndex(WorldGridPosition const & pos) const
	{
		SGW_ASSERT(validPosition(pos));
		return (pos.y - params_.minY) / params_.sizeY * chunksX_ + (pos.x - params_.minX) / params_.sizeX;
	}


	/*
	 * Returns which chunk the specified *world* position belongs to.
	 *
	 * @param pos Position to check
	 */
	WorldGridPosition chunkPosition(WorldGridPosition const & pos) const
	{
		SGW_ASSERT(validPosition(pos));
		return WorldGridPosition(
			(pos.x - params_.minX) / params_.sizeX,
			(pos.y - params_.minY) / params_.sizeY
		);
	}

	
	/*
	 * Checks if the specified chunk index is valid.
	 *
	 * @param x X position of chunk
	 * @param y Y position of chunk
	 */
	bool validChunk(int x, int y)
	{
		return x >= 0 && x < chunksX_ && y >= 0 && y < chunksY_;
	}

	
	/*
	 * Returns the index of the specified chunk.
	 *
	 * @param x X position of chunk
	 * @param y Y position of chunk
	 */
	unsigned int chunkPosIndex(int x, int y)
	{
		SGW_ASSERT(validChunk(x, y));
		return y * chunksX_ + x;
	}

	
	/*
	 * Returns the specified world chunk.
	 *
	 * @param index Chunk index
	 */
	Chunk & chunk(unsigned int index)
	{
		SGW_ASSERT(index <= chunks_.size());
		return chunks_[index];
	}


	/*
	 * Adds the specified object to the world grid.
	 *
	 * @param object Object to add
	 */
	void add(typename _T::Ptr object)
	{
		SGW_ASSERT(object->witnesses_.empty());
		SGW_ASSERT(object->visionExceptions_.empty());
		SGW_ASSERT(object->grid_ == nullptr);
		chunk(chunkIndex(object->gridPosition())).insert(object);
		object->grid_ = this;
		makeVisible(object);
		if (object->isAWitness())
		{
			addNeighbors(*object, object->gridPosition());
		}
	}
	

	/*
	 * Removes the specified object from the world grid.
	 *
	 * @param object Object to remove
	 */
	void remove(typename _T::Ptr object)
	{
		SGW_ASSERT(object->grid_ == this);
		if (object->isAWitness())
		{
			removeNeighbors(*object, object->gridPosition(), true);
			for (auto & exception : object->visionExceptions_)
			{
				object->onEntityInvisible(exception, true);
			}
		}

		for (auto & witness : object->witnesses_)
		{
			witness.lock()->onEntityInvisible(witness, true);
		}

		clearVisionData(object);
		makeInvisible(object, true);
		chunk(chunkIndex(object->gridPosition())).remove(object);
		object->visionExceptions_.clear();
		object->witnesses_.clear();
		object->grid_ = nullptr;
	}


	/*
	 * Calls the visitor function for each witness that can currently see the object.
	 *
	 * @param object  Object to check
	 * @param visitor Visitor callback function
	 */
	template <typename Func>
	void visitWitnesses(typename _T::Ptr object, Func visitor)
	{
		SGW_ASSERT(object->grid_ == this);
		WorldGridPosition chunkPos = chunkPosition(object->gridPosition());
		// Visit each chunk within the vision distance of the entity
		for (int x = std::max((int)0, chunkPos.x - params_.visionDistance); 
			x < std::min((int)chunksX_ - 1, chunkPos.x + params_.visionDistance); x++)
		{
			for (int y = std::max((int)0, chunkPos.y - params_.visionDistance); 
				y < std::min((int)chunksY_ - 1, chunkPos.y + params_.visionDistance); y++)
			{
				for (auto & witness : chunk(chunkPosIndex(x, y)).witnesses)
				{
					visitor(witness);
				}
			}
		}

		// Visit witnesses that are out of the vision distance but
		// can still see the entity due to hysteresis
		for (auto & witness : object->witnesses_)
		{
			visitor(witness);
		}
	}
	

	/*
	 * Updates the position of the object on the grid.
	 * This must be called before the actual position is updated, 
	 * so gridPosition() should still return the old position.
	 *
	 * @param object      Object being moved
	 * @param newPosition New object position
	 */
	void move(typename _T::Ptr object, WorldGridPosition const & newPosition)
	{
		// Avoid dereferencing the shared_ptr multiple times
		_T & o = *object;

		unsigned int oldIndex = chunkIndex(o.gridPosition());
		unsigned int newIndex = chunkIndex(newPosition);

		// Object didn't move to a new chunk --> don't update anything
		if (oldIndex == newIndex)
		{
			return;
		}

		WorldGridPosition oldPos = chunkPosition(o.gridPosition());
		WorldGridPosition newPos = chunkPosition(newPosition);

		bool isWitness = o.isAWitness();

		// Check if the object crossed more than 1 chunk in any axis
		if ((oldPos.x > newPos.x ? oldPos.x - newPos.x : newPos.x - oldPos.x) > 1 ||
			(oldPos.y > newPos.y ? oldPos.y - newPos.y : newPos.y - oldPos.y) > 1)
		{
			for (auto & exception : o.visionExceptions_)
			{
				o.onEntityInvisible(exception, false);
			}

			for (auto & witness : o.witnesses_)
			{
				witness.lock()->onEntityInvisible(object, false);
			}
			
			if (isWitness)
			{
				removeNeighbors(o, o.gridPosition(), false);
			}
			clearVisionData(object);
			if (isWitness)
			{
				addNeighbors(o, newPosition);
			}
			WARN("PERFORMANCE WARNING: Visibility data reset for %d", o.entityId());
		}
		else
		{
			std::vector<unsigned int> visible;
			int xMul = (oldPos.x - newPos.x);
			int yMul = (oldPos.y - newPos.y);

			// Find entities that left the vision range of this entity
			std::vector<unsigned int> leftRange;
			findChunks(leftRange, oldPos, params_.visionDistance, params_.visionDistance, xMul, yMul);

			// If hysteresis is enabled, collect entities on the borders of the visible region and
			// mark them as witnesses/non-witnesses
			if (params_.hysteresis > 0)
			{
				if (isWitness)
				{
					// Find entities that left the hysteresis range of this entity
					std::vector<unsigned int> leftHysteresis;
					findChunks(leftHysteresis, oldPos, params_.visionDistance, params_.visionDistance + params_.hysteresis, xMul, yMul);

					// Remove all objects that are outside our current hysteresis range
					for (unsigned int index : leftHysteresis)
					{
						for (auto & ent : chunk(index).objects)
						{
							auto it = o.visionExceptions_.find(ent);
							if (it != o.visionExceptions_.end())
							{
#ifdef DEBUG_WORLD_GRID
								typename _T::Ptr p = it->lock();
								TRACE("(%d) Entity %d left AoI (hysteresis)", o.entityId(), p->entityId());
#endif
								o.onEntityInvisible(ent, false);
								removeVisionException(object, ent.lock());
							}

							auto witnessIt = o.witnesses_.find(ent);
							if (witnessIt != o.witnesses_.end())
							{
								auto entPtr = ent.lock();
								entPtr->onEntityInvisible(object, false);
								removeVisionException(entPtr, object);
							}
						}
					}
				}
			
				// Add vision exceptions for objects that just entered hysteresis range
				// (visionRange < distance <= visionRange + hysteresis)
				for (unsigned int index : leftRange)
				{
					if (isWitness)
					{
						for (auto & ent : chunk(index).objects)
						{
							typename _T::Ptr p = ent.lock();
							addVisionException(object, p);
						}
					}

					for (auto & ent : chunk(index).witnesses)
					{
						typename _T::Ptr p = ent.lock();
						addVisionException(p, object);
					}
				}
			}
			else
			{
				// Hide entities that left the vision range of this object
				// (visionRange < distance)
				for (unsigned int index : leftRange)
				{
					if (isWitness)
					{
						for (auto & ent : chunk(index).objects)
						{
#ifdef DEBUG_WORLD_GRID
							typename _T::Ptr p = ent.lock();
							TRACE("(%d) Entity %d left AoI", o.entityId(), p->entityId());
#endif
							o.onEntityInvisible(ent, false);
						}
					}
					
					for (auto & ent : chunk(index).witnesses)
					{
						typename _T::Ptr p = ent.lock();
#ifdef DEBUG_WORLD_GRID
						TRACE("(%d) Entity %d left AoI", p->entityId(), o.entityId());
#endif
						p->onEntityInvisible(object, false);
					}
				}
			}
			
			// Find entities that entered the vision range of this entity
			findChunks(visible, newPos, params_.visionDistance, params_.visionDistance, -xMul, -yMul);

			// Remove newly visible objects from the witness list
			// Notify the entity that it sees new objects
			for (unsigned int index : visible)
			{
				if (isWitness)
				{
					for (auto & ent : chunk(index).objects)
					{
						if (params_.hysteresis > 0)
						{
							auto it = o.visionExceptions_.find(ent);
							if (it != o.visionExceptions_.end())
							{
								removeVisionException(object, ent.lock());
							}
							else
							{
#ifdef DEBUG_WORLD_GRID
								typename _T::Ptr p = ent.lock();
								TRACE("(%d) Entity %d entered AoI (no hysteresis)", o.entityId(), p->entityId());
#endif
								o.onEntityVisible(ent);
							}
						}
						else
						{
#ifdef DEBUG_WORLD_GRID
							typename _T::Ptr p = ent.lock();
							TRACE("(%d) Entity %d entered AoI", o.entityId(), p->entityId());
#endif
							o.onEntityVisible(ent);
						}
					}
				}
				
				for (auto & ent : chunk(index).witnesses)
				{
					typename _T::Ptr p = ent.lock();
					if (params_.hysteresis > 0)
					{
						auto it = p->visionExceptions_.find(object);
						if (it != p->visionExceptions_.end())
						{
							removeVisionException(p, object);
						}
						else
						{
#ifdef DEBUG_WORLD_GRID
							TRACE("(%d) Entity %d entered AoI (no hysteresis)", p->entityId(), o.entityId());
#endif
							p->onEntityVisible(object);
						}
					}
					else
					{
#ifdef DEBUG_WORLD_GRID
						TRACE("(%d) Entity %d entered AoI", p->entityId(), o.entityId());
#endif
						p->onEntityVisible(object);
					}
				}
			}
		}

		// Remove object from its old chunk
		chunk(oldIndex).remove(object);
		
		// Add object to the new chunk
		chunk(newIndex).insert(object);
	}


private:
	Params params_;
	int chunksX_, chunksY_;
	typename std::vector<Chunk> chunks_;

	/*
	 * Returns the indices of chunks that match the search criteria.
	 * Example:
	 * +>-----------+   Params:
	 * v   XXXXXX   |   distance = 2
	 * v   .....X   |   iterDistance = 3
	 * |   .....X   |   xDir = 1
	 * |   ..P..X   |   yDir = -1
	 * |   .....X   |   P : pos
	 * |   .....X   |   X : matched chunks
	 * +------------+   . : chunks in range
	 * 
	 * @param chunks       List of matching chunk indices
	 * @param pos          Search for chunks around this chunk position
	 * @param distance     Search distance on the constant axis
	 * @param iterDistance Search distance on the changed axis
	 * @param xDir         Direction of movement (X axis)
	 * @param yDir         Direction of movement (Y axis)
	 */
	void findChunks(std::vector<unsigned int> & chunks, WorldGridPosition const & pos, int distance, int iterDistance, int xDir, int yDir)
	{
		if (xDir)
		{
			int x = pos.x + iterDistance * xDir;
			if (x >= 0 && x < chunksX_)
			{
				for (int y = std::max(pos.y - distance, (int)0); y <= std::min(pos.y + distance, (int)chunksY_ - 1); y++)
				{
					if (validChunk(x, y))
					{
						chunks.push_back(chunkPosIndex(x, y));
					}
				}
			}
		}

		if (yDir)
		{
			int y = pos.y + iterDistance * yDir;
			if (y >= 0 && y < chunksY_)
			{
				for (int x = std::max(pos.x - distance, (int)0); x <= std::min(pos.x + distance, (int)chunksX_ - 1); x++)
				{
					if (validChunk(x, y) && (!xDir || std::find(chunks.begin(), chunks.end(), chunkPosIndex(x, y)) == chunks.end()))
					{
						chunks.push_back(chunkPosIndex(x, y));
					}
				}
			}
		}
	}

	/*
	 * Makes the entity visible to every other entity in the neighbouring cells.
	 */
	void makeVisible(typename _T::Ptr object)
	{
		WorldGridPosition chunkPos = chunkPosition(object->gridPosition());
		for (int x = std::max((int)0, chunkPos.x - params_.visionDistance); 
			x <= std::min((int)chunksX_ - 1, chunkPos.x + params_.visionDistance); x++)
		{
			for (int y = std::max((int)0, chunkPos.y - params_.visionDistance); 
				y <= std::min((int)chunksY_ - 1, chunkPos.y + params_.visionDistance); y++)
			{
				for (auto & witness : chunk(chunkPosIndex(x, y)).witnesses)
				{
					witness.lock()->onEntityVisible(object);
				}
			}
		}
	}

	/*
	 * Makes the entity invisible to every other entity in the neighbouring cells.
	 */
	void makeInvisible(typename _T::Ptr object, bool destroyed)
	{
		WorldGridPosition chunkPos = chunkPosition(object->gridPosition());
		for (int x = std::max((int)0, chunkPos.x - params_.visionDistance); 
			x <= std::min((int)chunksX_ - 1, chunkPos.x + params_.visionDistance); x++)
		{
			for (int y = std::max((int)0, chunkPos.y - params_.visionDistance); 
				y <= std::min((int)chunksY_ - 1, chunkPos.y + params_.visionDistance); y++)
			{
				for (auto & witness : chunk(chunkPosIndex(x, y)).witnesses)
				{
					witness.lock()->onEntityInvisible(object, destroyed);
				}
			}
		}
	}

	/*
	 * Searches the neighbouring cells of the entity for visible objects
	 * and makes them visible for the entity.
	 */
	void addNeighbors(_T & object, WorldGridPosition const & pos)
	{
		WorldGridPosition chunkPos = chunkPosition(pos);
		for (int x = std::max((int)0, chunkPos.x - params_.visionDistance); 
			x <= std::min((int)chunksX_ - 1, chunkPos.x + params_.visionDistance); x++)
		{
			for (int y = std::max((int)0, chunkPos.y - params_.visionDistance); 
				y <= std::min((int)chunksY_ - 1, chunkPos.y + params_.visionDistance); y++)
			{
				for (auto & ent : chunk(chunkPosIndex(x, y)).objects)
				{
					object.onEntityVisible(ent);
				}
			}
		}
	}

	/*
	 * Searches the neighbouring cells of the entity for visible objects
	 * and makes them invisible for the entity.
	 */
	void removeNeighbors(_T & object, WorldGridPosition const & pos, bool destroyed)
	{
		WorldGridPosition chunkPos = chunkPosition(pos);
		for (int x = std::max((int)0, chunkPos.x - params_.visionDistance); 
			x <= std::min((int)chunksX_ - 1, chunkPos.x + params_.visionDistance); x++)
		{
			for (int y = std::max((int)0, chunkPos.y - params_.visionDistance); 
				y <= std::min((int)chunksY_ - 1, chunkPos.y + params_.visionDistance); y++)
			{
				for (auto & ent : chunk(chunkPosIndex(x, y)).objects)
				{
					object.onEntityInvisible(ent, destroyed);
				}
			}
		}
	}

	/*
	 * Adds an object to the vision exception list (list of out of range entities that it sees)
	 * of the witness and vice versa.
	 */
	void addVisionException(typename _T::Ptr witness, typename _T::Ptr object)
	{
		SGW_ASSERT(witness != object);
		SGW_ASSERT(object->witnesses_.find(witness) == object->witnesses_.end());
		SGW_ASSERT(witness->visionExceptions_.find(object) == witness->visionExceptions_.end());
		object->witnesses_.insert(witness);
		witness->visionExceptions_.insert(object);
#ifdef DEBUG_WORLD_GRID
		TRACE("Added vision exception for %d on entity %d", witness->entityId(), object->entityId());
#endif
	}
	
	/*
	 * Removes an object from the vision exception list (list of out of range entities that it sees)
	 * of the witness and vice versa.
	 */
	void removeVisionException(typename _T::Ptr witness, typename _T::Ptr object)
	{
		SGW_ASSERT(witness != object);
		auto excIt = witness->visionExceptions_.find(object);
		SGW_ASSERT(excIt != witness->visionExceptions_.end());
		witness->visionExceptions_.erase(excIt);

		auto witnessIt = object->witnesses_.find(witness);
		SGW_ASSERT(witnessIt != object->witnesses_.end());
		object->witnesses_.erase(witnessIt);
		
#ifdef DEBUG_WORLD_GRID
		TRACE("Removed vision exception for %d on entity %d", witness->entityId(), object->entityId());
#endif
	}
	
	/*
	 * Removes an object from the vision exception list (list of out of range entities that it sees)
	 * of the witness and vice versa.
	 */
	void clearVisionData(typename _T::Ptr object)
	{
		_T & o = *object;

		while (!o.visionExceptions_.empty())
		{
			removeVisionException(object, o.visionExceptions_.rbegin()->lock());
		}

		while (!o.witnesses_.empty())
		{
			removeVisionException(o.witnesses_.rbegin()->lock(), object);
		}
	}
};

