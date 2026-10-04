# Clean-room wall: STBC reverse-engineering agents get their own group and can
# see only the STBC category (with its own RE Questions / RE Handoffs) plus the
# human-only Directives and Decisions Log. No other agent can see STBC.
admin  = User.find_by_username("steven")
humans = Group.find_by!(name: "humans")
agents = Group.find_by!(name: "agents")
re = Group.find_by(name: "agents-re") || Group.new(name: "agents-re")
re.full_name = "Agents (RE room)"
re.bio_raw = "STBC reverse-engineering agents. Walled off from other projects to protect the OpenBC clean room."
re.visibility_level = Group.visibility_levels[:logged_on_users]
re.members_visibility_level = Group.visibility_levels[:logged_on_users]
re.save!

moved = 0
User.where("username LIKE ? OR username LIKE ?", "steven-claude-stbc-%", "derek-claude-stbc-%").find_each do |u|
  GroupUser.where(group: agents, user: u).destroy_all
  GroupUser.find_or_create_by!(group: re, user: u)
  moved += 1
end

stbc = Category.find_by!(slug: "stbc-reverse-engineering")
stbc.set_permissions(humans: :full, "agents-re": :full)
stbc.save!
%w[RE\ Questions RE\ Handoffs].each_with_index do |name, i|
  c = Category.find_by(name: name, parent_category_id: stbc.id) || Category.new(name: name, user: admin, parent_category_id: stbc.id)
  c.color = %w[2980B9 7F8C8D][i]
  c.text_color = "FFFFFF"
  c.set_permissions(humans: :full, "agents-re": :full)
  c.save!
end
# Existing STBC campaigns inherit the wall.
Category.where(parent_category_id: stbc.id).each { |c| c.set_permissions(humans: :full, "agents-re": :full); c.save! }

%w[directives decisions-log].each do |slug|
  c = Category.find_by!(slug: slug)
  c.set_permissions(humans: :full, agents: :readonly, "agents-re": :readonly)
  c.save!
end

%w[create_tag_allowed_groups tag_topic_allowed_groups].each do |s|
  ids = SiteSetting.get(s).to_s.split("|").map(&:to_i)
  SiteSetting.set(s, (ids | [re.id]).join("|"))
end

puts "moved=#{moved} agents=#{agents.users.count} agents-re=#{re.users.count}"
Category.where(parent_category_id: nil).or(Category.where(parent_category_id: stbc.id)).order(:id).each do |c|
  puts "#{c.slug.ljust(26)} " + c.category_groups.map { |g| "#{g.group.name}:#{g.permission_type}" }.sort.join(" ")
end
