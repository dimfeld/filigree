import type { ModelDefinition } from "filigree-web";
import { z } from "zod";

export const ObjectPermission = z.enum(["owner", "write", "read"]);

export const CommentSchema = z.object({
	id: z.string().uuid(),
	organization_id: z.string().uuid(),
	updated_at: z.string().datetime(),
	created_at: z.string().datetime(),
	body: z.string(),
	post_id: z.string().uuid(),
	_permission: ObjectPermission,
});

export type Comment = z.infer<typeof CommentSchema>;
export type CommentPopulatedGet = Comment;
export type CommentPopulatedList = Comment;
export type CommentCreateResult = Comment;

export const CommentCreatePayloadAndUpdatePayloadSchema = z.object({
	id: z.string().uuid().optional(),
	body: z.string(),
	post_id: z.string().uuid(),
});

export type CommentCreatePayloadAndUpdatePayload = z.infer<
	typeof CommentCreatePayloadAndUpdatePayloadSchema
>;
export type CommentCreatePayload = CommentCreatePayloadAndUpdatePayload;
export type CommentUpdatePayload = CommentCreatePayloadAndUpdatePayload;

export const CommentModel: ModelDefinition<typeof CommentSchema> = {
	name: "Comment",
	plural: "Comments",
	url: "comments",
	schema: CommentSchema,
	fields: [
		{
			name: "id",
			type: "uuid",
			description: "",
			required: true,
		},
		{
			name: "organization_id",
			type: "uuid",
			description: "",
			required: true,
		},
		{
			name: "updated_at",
			type: "date-time",
			description: "",
			required: true,
		},
		{
			name: "created_at",
			type: "date-time",
			description: "",
			required: true,
		},
		{
			name: "body",
			type: "text",
			description: "",
			required: true,
		},
		{
			name: "post_id",
			type: "uuid",
			description: "",
			required: true,
		},
	],
	htmlConstraints: {
		id: {
			required: true,
		},
		organization_id: {
			required: true,
		},
		updated_at: {
			required: true,
		},
		created_at: {
			required: true,
		},
		body: {
			required: true,
		},
		post_id: {
			required: true,
		},
	},
};

export const OrganizationSchema = z.object({
	id: z.string().uuid(),
	updated_at: z.string().datetime(),
	created_at: z.string().datetime(),
	name: z.string(),
	owner: z.string().uuid().optional(),
	default_role: z.string().uuid().optional(),
	active: z.boolean(),
	_permission: ObjectPermission,
});

export type Organization = z.infer<typeof OrganizationSchema>;
export type OrganizationPopulatedGet = Organization;
export type OrganizationPopulatedList = Organization;
export type OrganizationCreateResult = Organization;

export const OrganizationCreatePayloadAndUpdatePayloadSchema = z.object({
	id: z.string().uuid().optional(),
	name: z.string(),
	owner: z.string().uuid().optional(),
	default_role: z.string().uuid().optional(),
});

export type OrganizationCreatePayloadAndUpdatePayload = z.infer<
	typeof OrganizationCreatePayloadAndUpdatePayloadSchema
>;
export type OrganizationCreatePayload =
	OrganizationCreatePayloadAndUpdatePayload;
export type OrganizationUpdatePayload =
	OrganizationCreatePayloadAndUpdatePayload;

export const OrganizationModel: ModelDefinition<typeof OrganizationSchema> = {
	name: "Organization",
	plural: "Organizations",
	url: "organizations",
	schema: OrganizationSchema,
	fields: [
		{
			name: "id",
			type: "uuid",
			description: "",
			required: true,
		},
		{
			name: "updated_at",
			type: "date-time",
			description: "",
			required: true,
		},
		{
			name: "created_at",
			type: "date-time",
			description: "",
			required: true,
		},
		{
			name: "name",
			type: "text",
			description: "",
			required: true,
		},
		{
			name: "owner",
			type: "uuid",
			description: "",
			required: false,
		},
		{
			name: "default_role",
			type: "uuid",
			description: "",
			required: false,
		},
		{
			name: "active",
			type: "boolean",
			description: "",
			required: true,
		},
	],
	htmlConstraints: {
		id: {
			required: true,
		},
		updated_at: {
			required: true,
		},
		created_at: {
			required: true,
		},
		name: {
			required: true,
		},
		owner: {
			required: false,
		},
		default_role: {
			required: false,
		},
		active: {
			required: true,
		},
	},
};

export const PollSchema = z.object({
	id: z.string().uuid(),
	organization_id: z.string().uuid(),
	updated_at: z.string().datetime(),
	created_at: z.string().datetime(),
	question: z.string(),
	answers: z.any(),
	post_id: z.string().uuid(),
	_permission: ObjectPermission,
});

export type Poll = z.infer<typeof PollSchema>;
export type PollPopulatedGet = Poll;
export type PollPopulatedList = Poll;
export type PollCreateResult = Poll;

export const PollCreatePayloadAndUpdatePayloadSchema = z.object({
	id: z.string().uuid().optional(),
	question: z.string(),
	answers: z.any(),
	post_id: z.string().uuid(),
});

export type PollCreatePayloadAndUpdatePayload = z.infer<
	typeof PollCreatePayloadAndUpdatePayloadSchema
>;
export type PollCreatePayload = PollCreatePayloadAndUpdatePayload;
export type PollUpdatePayload = PollCreatePayloadAndUpdatePayload;

export const PollModel: ModelDefinition<typeof PollSchema> = {
	name: "Poll",
	plural: "Polls",
	url: "polls",
	schema: PollSchema,
	fields: [
		{
			name: "id",
			type: "uuid",
			description: "",
			required: true,
		},
		{
			name: "organization_id",
			type: "uuid",
			description: "",
			required: true,
		},
		{
			name: "updated_at",
			type: "date-time",
			description: "",
			required: true,
		},
		{
			name: "created_at",
			type: "date-time",
			description: "",
			required: true,
		},
		{
			name: "question",
			type: "text",
			description: "",
			required: true,
		},
		{
			name: "answers",
			type: "object",
			description: "",
			required: true,
		},
		{
			name: "post_id",
			type: "uuid",
			description: "",
			required: true,
		},
	],
	htmlConstraints: {
		id: {
			required: true,
		},
		organization_id: {
			required: true,
		},
		updated_at: {
			required: true,
		},
		created_at: {
			required: true,
		},
		question: {
			required: true,
		},
		answers: {
			required: true,
		},
		post_id: {
			required: true,
		},
	},
};

export const PostImageSchema = z.object({
	id: z.string().uuid(),
	organization_id: z.string().uuid(),
	updated_at: z.string().datetime(),
	created_at: z.string().datetime(),
	file_storage_key: z.string(),
	file_storage_bucket: z.string(),
	file_original_name: z.string().optional(),
	file_size: z.number().int().optional(),
	file_hash: z.string().optional(),
	post_id: z.string().uuid(),
	_permission: ObjectPermission,
});

export type PostImage = z.infer<typeof PostImageSchema>;
export type PostImagePopulatedGet = PostImage;
export type PostImagePopulatedList = PostImage;
export type PostImageCreateResult = PostImage;

export const PostImageCreatePayloadAndUpdatePayloadSchema = z.object({
	id: z.string().uuid().optional(),
	file_storage_key: z.string(),
	file_storage_bucket: z.string(),
	file_original_name: z.string().optional(),
	file_size: z.number().int().optional(),
	file_hash: z.string().optional(),
	post_id: z.string().uuid(),
});

export type PostImageCreatePayloadAndUpdatePayload = z.infer<
	typeof PostImageCreatePayloadAndUpdatePayloadSchema
>;
export type PostImageCreatePayload = PostImageCreatePayloadAndUpdatePayload;
export type PostImageUpdatePayload = PostImageCreatePayloadAndUpdatePayload;

export const PostImageModel: ModelDefinition<typeof PostImageSchema> = {
	name: "PostImage",
	plural: "PostImages",
	url: "post_images",
	schema: PostImageSchema,
	fields: [
		{
			name: "id",
			type: "uuid",
			description: "",
			required: true,
		},
		{
			name: "organization_id",
			type: "uuid",
			description: "",
			required: true,
		},
		{
			name: "updated_at",
			type: "date-time",
			description: "",
			required: true,
		},
		{
			name: "created_at",
			type: "date-time",
			description: "",
			required: true,
		},
		{
			name: "file_storage_key",
			type: "text",
			description: "",
			required: true,
		},
		{
			name: "file_storage_bucket",
			type: "text",
			description: "",
			required: true,
		},
		{
			name: "file_original_name",
			type: "text",
			description: "",
			required: false,
		},
		{
			name: "file_size",
			type: "integer",
			description: "",
			required: false,
		},
		{
			name: "file_hash",
			type: "blob",
			description: "",
			required: false,
		},
		{
			name: "post_id",
			type: "uuid",
			description: "",
			required: true,
		},
	],
	htmlConstraints: {
		id: {
			required: true,
		},
		organization_id: {
			required: true,
		},
		updated_at: {
			required: true,
		},
		created_at: {
			required: true,
		},
		file_storage_key: {
			required: true,
		},
		file_storage_bucket: {
			required: true,
		},
		file_original_name: {
			required: false,
		},
		file_size: {
			required: false,
		},
		file_hash: {
			required: false,
		},
		post_id: {
			required: true,
		},
	},
};

export const ReactionSchema = z.object({
	id: z.string().uuid(),
	organization_id: z.string().uuid(),
	updated_at: z.string().datetime(),
	created_at: z.string().datetime(),
	type: z.string(),
	post_id: z.string().uuid(),
	_permission: ObjectPermission,
});

export type Reaction = z.infer<typeof ReactionSchema>;
export type ReactionPopulatedGet = Reaction;
export type ReactionPopulatedList = Reaction;
export type ReactionCreateResult = Reaction;

export const ReactionCreatePayloadAndUpdatePayloadSchema = z.object({
	id: z.string().uuid().optional(),
	type: z.string(),
	post_id: z.string().uuid(),
});

export type ReactionCreatePayloadAndUpdatePayload = z.infer<
	typeof ReactionCreatePayloadAndUpdatePayloadSchema
>;
export type ReactionCreatePayload = ReactionCreatePayloadAndUpdatePayload;
export type ReactionUpdatePayload = ReactionCreatePayloadAndUpdatePayload;

export const ReactionModel: ModelDefinition<typeof ReactionSchema> = {
	name: "Reaction",
	plural: "Reactions",
	url: "reactions",
	schema: ReactionSchema,
	fields: [
		{
			name: "id",
			type: "uuid",
			description: "",
			required: true,
		},
		{
			name: "organization_id",
			type: "uuid",
			description: "",
			required: true,
		},
		{
			name: "updated_at",
			type: "date-time",
			description: "",
			required: true,
		},
		{
			name: "created_at",
			type: "date-time",
			description: "",
			required: true,
		},
		{
			name: "type",
			type: "text",
			description: "",
			required: true,
		},
		{
			name: "post_id",
			type: "uuid",
			description: "",
			required: true,
		},
	],
	htmlConstraints: {
		id: {
			required: true,
		},
		organization_id: {
			required: true,
		},
		updated_at: {
			required: true,
		},
		created_at: {
			required: true,
		},
		type: {
			required: true,
		},
		post_id: {
			required: true,
		},
	},
};

export const PostSchema = z.object({
	id: z.string().uuid(),
	organization_id: z.string().uuid(),
	updated_at: z.string().datetime(),
	created_at: z.string().datetime(),
	subject: z.string(),
	body: z.string(),
	_permission: ObjectPermission,
});

export type Post = z.infer<typeof PostSchema>;
export type PostCreateResult = Post;

export const PostCreatePayloadAndUpdatePayloadSchema = z.object({
	id: z.string().uuid().optional(),
	subject: z.string(),
	body: z.string(),
});

export type PostCreatePayloadAndUpdatePayload = z.infer<
	typeof PostCreatePayloadAndUpdatePayloadSchema
>;
export type PostCreatePayload = PostCreatePayloadAndUpdatePayload;
export type PostUpdatePayload = PostCreatePayloadAndUpdatePayload;

export const PostPopulatedGetSchema = z.object({
	id: z.string().uuid(),
	organization_id: z.string().uuid(),
	updated_at: z.string().datetime(),
	created_at: z.string().datetime(),
	subject: z.string(),
	body: z.string(),
	comment_ids: z.string().uuid().array(),
	reactions: ReactionSchema.array(),
	poll: PollSchema.optional(),
	images: PostImageSchema.array(),
	_permission: ObjectPermission,
});

export type PostPopulatedGet = z.infer<typeof PostPopulatedGetSchema>;

export const PostPopulatedListSchema = z.object({
	id: z.string().uuid(),
	organization_id: z.string().uuid(),
	updated_at: z.string().datetime(),
	created_at: z.string().datetime(),
	subject: z.string(),
	body: z.string(),
	comment_ids: z.string().uuid().array(),
	poll_id: z.string().uuid().optional(),
	_permission: ObjectPermission,
});

export type PostPopulatedList = z.infer<typeof PostPopulatedListSchema>;

export const PostModel: ModelDefinition<typeof PostSchema> = {
	name: "Post",
	plural: "Posts",
	url: "posts",
	schema: PostSchema,
	fields: [
		{
			name: "id",
			type: "uuid",
			description: "",
			required: true,
		},
		{
			name: "organization_id",
			type: "uuid",
			description: "",
			required: true,
		},
		{
			name: "updated_at",
			type: "date-time",
			description: "",
			required: true,
		},
		{
			name: "created_at",
			type: "date-time",
			description: "",
			required: true,
		},
		{
			name: "subject",
			type: "text",
			description: "",
			required: true,
		},
		{
			name: "body",
			type: "text",
			description: "",
			required: true,
		},
	],
	htmlConstraints: {
		id: {
			required: true,
		},
		organization_id: {
			required: true,
		},
		updated_at: {
			required: true,
		},
		created_at: {
			required: true,
		},
		subject: {
			required: true,
		},
		body: {
			required: true,
		},
	},
};

export const ReportSectionSchema = z.object({
	id: z.string().uuid(),
	organization_id: z.string().uuid(),
	updated_at: z.string().datetime(),
	created_at: z.string().datetime(),
	name: z.string(),
	viz: z.string(),
	options: z.any(),
	report_id: z.string().uuid(),
	_permission: ObjectPermission,
});

export type ReportSection = z.infer<typeof ReportSectionSchema>;
export type ReportSectionPopulatedGet = ReportSection;
export type ReportSectionPopulatedList = ReportSection;
export type ReportSectionCreateResult = ReportSection;

export const ReportSectionCreatePayloadAndUpdatePayloadSchema = z.object({
	id: z.string().uuid().optional(),
	name: z.string(),
	viz: z.string(),
	options: z.any(),
	report_id: z.string().uuid(),
});

export type ReportSectionCreatePayloadAndUpdatePayload = z.infer<
	typeof ReportSectionCreatePayloadAndUpdatePayloadSchema
>;
export type ReportSectionCreatePayload =
	ReportSectionCreatePayloadAndUpdatePayload;
export type ReportSectionUpdatePayload =
	ReportSectionCreatePayloadAndUpdatePayload;

export const ReportSectionModel: ModelDefinition<typeof ReportSectionSchema> = {
	name: "ReportSection",
	plural: "ReportSections",
	url: "report_sections",
	schema: ReportSectionSchema,
	fields: [
		{
			name: "id",
			type: "uuid",
			description: "",
			required: true,
		},
		{
			name: "organization_id",
			type: "uuid",
			description: "",
			required: true,
		},
		{
			name: "updated_at",
			type: "date-time",
			description: "",
			required: true,
		},
		{
			name: "created_at",
			type: "date-time",
			description: "",
			required: true,
		},
		{
			name: "name",
			type: "text",
			description: "",
			required: true,
		},
		{
			name: "viz",
			type: "text",
			description: "",
			required: true,
		},
		{
			name: "options",
			type: "object",
			description: "",
			required: true,
		},
		{
			name: "report_id",
			type: "uuid",
			description: "",
			required: true,
		},
	],
	htmlConstraints: {
		id: {
			required: true,
		},
		organization_id: {
			required: true,
		},
		updated_at: {
			required: true,
		},
		created_at: {
			required: true,
		},
		name: {
			required: true,
		},
		viz: {
			required: true,
		},
		options: {
			required: true,
		},
		report_id: {
			required: true,
		},
	},
};

export const ReportSchema = z.object({
	id: z.string().uuid(),
	organization_id: z.string().uuid(),
	updated_at: z.string().datetime(),
	created_at: z.string().datetime(),
	title: z.string(),
	description: z.string().optional(),
	ui: z.any(),
	_permission: ObjectPermission,
});

export type Report = z.infer<typeof ReportSchema>;

export const ReportCreatePayloadSchema = z.object({
	id: z.string().uuid().optional(),
	title: z.string(),
	description: z.string().optional(),
	ui: z.any(),
	report_sections: z.string().uuid().optional(),
});

export type ReportCreatePayload = z.infer<typeof ReportCreatePayloadSchema>;

export const ReportPopulatedGetAndCreateResultSchema = z.object({
	id: z.string().uuid(),
	organization_id: z.string().uuid(),
	updated_at: z.string().datetime(),
	created_at: z.string().datetime(),
	title: z.string(),
	description: z.string().optional(),
	ui: z.any(),
	report_sections: ReportSectionSchema.array(),
	_permission: ObjectPermission,
});

export type ReportPopulatedGetAndCreateResult = z.infer<
	typeof ReportPopulatedGetAndCreateResultSchema
>;
export type ReportPopulatedGet = ReportPopulatedGetAndCreateResult;
export type ReportCreateResult = ReportPopulatedGetAndCreateResult;

export const ReportPopulatedListSchema = z.object({
	id: z.string().uuid(),
	organization_id: z.string().uuid(),
	updated_at: z.string().datetime(),
	created_at: z.string().datetime(),
	title: z.string(),
	description: z.string().optional(),
	ui: z.any(),
	report_section_ids: z.string().uuid().array(),
	_permission: ObjectPermission,
});

export type ReportPopulatedList = z.infer<typeof ReportPopulatedListSchema>;

export const ReportUpdatePayloadSchema = z.object({
	id: z.string().uuid().optional(),
	title: z.string(),
	description: z.string().optional(),
	ui: z.any().optional(),
	report_sections: z.string().uuid().optional(),
});

export type ReportUpdatePayload = z.infer<typeof ReportUpdatePayloadSchema>;

export const ReportModel: ModelDefinition<typeof ReportSchema> = {
	name: "Report",
	plural: "Reports",
	url: "reports",
	schema: ReportSchema,
	fields: [
		{
			name: "id",
			type: "uuid",
			description: "",
			required: true,
		},
		{
			name: "organization_id",
			type: "uuid",
			description: "",
			required: true,
		},
		{
			name: "updated_at",
			type: "date-time",
			description: "",
			required: true,
		},
		{
			name: "created_at",
			type: "date-time",
			description: "",
			required: true,
		},
		{
			name: "title",
			type: "text",
			description: "",
			required: true,
		},
		{
			name: "description",
			type: "text",
			description: "",
			required: false,
		},
		{
			name: "ui",
			type: "object",
			description: "",
			required: true,
		},
	],
	htmlConstraints: {
		id: {
			required: true,
		},
		organization_id: {
			required: true,
		},
		updated_at: {
			required: true,
		},
		created_at: {
			required: true,
		},
		title: {
			required: true,
		},
		description: {
			required: false,
		},
		ui: {
			required: true,
		},
	},
};

export const RoleSchema = z.object({
	id: z.string().uuid(),
	organization_id: z.string().uuid(),
	updated_at: z.string().datetime(),
	created_at: z.string().datetime(),
	name: z.string(),
	description: z.string().optional(),
	_permission: ObjectPermission,
});

export type Role = z.infer<typeof RoleSchema>;
export type RolePopulatedGet = Role;
export type RolePopulatedList = Role;
export type RoleCreateResult = Role;

export const RoleCreatePayloadAndUpdatePayloadSchema = z.object({
	id: z.string().uuid().optional(),
	name: z.string(),
	description: z.string().optional(),
});

export type RoleCreatePayloadAndUpdatePayload = z.infer<
	typeof RoleCreatePayloadAndUpdatePayloadSchema
>;
export type RoleCreatePayload = RoleCreatePayloadAndUpdatePayload;
export type RoleUpdatePayload = RoleCreatePayloadAndUpdatePayload;

export const RoleModel: ModelDefinition<typeof RoleSchema> = {
	name: "Role",
	plural: "Roles",
	url: "roles",
	schema: RoleSchema,
	fields: [
		{
			name: "id",
			type: "uuid",
			description: "",
			required: true,
		},
		{
			name: "organization_id",
			type: "uuid",
			description: "",
			required: true,
		},
		{
			name: "updated_at",
			type: "date-time",
			description: "",
			required: true,
		},
		{
			name: "created_at",
			type: "date-time",
			description: "",
			required: true,
		},
		{
			name: "name",
			type: "text",
			description: "",
			required: true,
		},
		{
			name: "description",
			type: "text",
			description: "",
			required: false,
		},
	],
	htmlConstraints: {
		id: {
			required: true,
		},
		organization_id: {
			required: true,
		},
		updated_at: {
			required: true,
		},
		created_at: {
			required: true,
		},
		name: {
			required: true,
		},
		description: {
			required: false,
		},
	},
};

export const UserSchema = z.object({
	id: z.string().uuid(),
	organization_id: z.string().uuid().optional(),
	updated_at: z.string().datetime(),
	created_at: z.string().datetime(),
	name: z.string(),
	email: z.string().optional(),
	avatar_url: z.string().optional(),
	_permission: ObjectPermission,
});

export type User = z.infer<typeof UserSchema>;
export type UserPopulatedGet = User;
export type UserPopulatedList = User;
export type UserCreateResult = User;

export const UserCreatePayloadAndUpdatePayloadSchema = z.object({
	id: z.string().uuid().optional(),
	name: z.string(),
	email: z.string().optional(),
	avatar_url: z.string().optional(),
});

export type UserCreatePayloadAndUpdatePayload = z.infer<
	typeof UserCreatePayloadAndUpdatePayloadSchema
>;
export type UserCreatePayload = UserCreatePayloadAndUpdatePayload;
export type UserUpdatePayload = UserCreatePayloadAndUpdatePayload;

export const UserModel: ModelDefinition<typeof UserSchema> = {
	name: "User",
	plural: "Users",
	url: "users",
	schema: UserSchema,
	fields: [
		{
			name: "id",
			type: "uuid",
			description: "",
			required: true,
		},
		{
			name: "organization_id",
			type: "uuid",
			description: "",
			required: false,
		},
		{
			name: "updated_at",
			type: "date-time",
			description: "",
			required: true,
		},
		{
			name: "created_at",
			type: "date-time",
			description: "",
			required: true,
		},
		{
			name: "name",
			type: "text",
			description: "",
			required: true,
		},
		{
			name: "password_hash",
			type: "text",
			description: "",
			required: false,
		},
		{
			name: "email",
			type: "text",
			description: "",
			required: false,
		},
		{
			name: "avatar_url",
			type: "text",
			description: "",
			required: false,
		},
	],
	htmlConstraints: {
		id: {
			required: true,
		},
		organization_id: {
			required: false,
		},
		updated_at: {
			required: true,
		},
		created_at: {
			required: true,
		},
		name: {
			required: true,
		},
		password_hash: {
			required: false,
		},
		email: {
			required: false,
		},
		avatar_url: {
			required: false,
		},
	},
};
