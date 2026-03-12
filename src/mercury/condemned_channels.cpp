#include <stdafx.hpp>
#include <mercury/condemned_channels.hpp>
#include <log/logger.hpp>

namespace Mercury
{

CondemnedChannels::CondemnedChannels(Nub & nub)
	: nub_(nub)
{
	timerId_ = nub_.timers().addTimer(
		[this](auto &, auto, uint64_t now, auto *) { pollChannels(now); },
		nub_.lastTick() + PollInterval,
		nullptr
	);
}

CondemnedChannels::~CondemnedChannels()
{
	nub_.timers().cancelTimer(timerId_);
}

void CondemnedChannels::addChannel(BaseChannel::Ptr channel)
{
	SGW_ASSERT(channel->condemned());
	channels_.push_back(channel);
}

void CondemnedChannels::pollChannels(uint64_t now)
{
	for (size_t i = 0; i < channels_.size(); )
	{
		auto & channel = *channels_[i];

		if (!channel.hasUnackedPackets() ||
			channel.lastActivity() < now + InactiveChannelTimeout ||
			channel.lastPeerActivity() < now + InactiveChannelTimeout)
		{
			TRACE("CondemnedChannels::pollChannels(%s): Closing channel", channel.address().address().to_string().c_str());
			channel.close();
			SGW_ASSERT(channel.closed());
		}

		if (channel.closed())
		{
			channels_.erase(channels_.begin() + i);
		}
		else
		{
			++i;
		}
	}

	timerId_ = nub_.timers().addTimer(
		[this](auto &, auto, uint64_t now, auto *) { pollChannels(now); },
		nub_.lastTick() + PollInterval,
		nullptr
	);
}

}