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
export const CommentPopulatedGetSchema = CommentSchema;
export type CommentPopulatedGet = Comment;
export const CommentPopulatedListSchema = CommentSchema;
export type CommentPopulatedList = Comment;
export const CommentCreateResultSchema = CommentSchema;
export type CommentCreateResult = Comment;

export const CommentCreatePayloadAndUpdatePayloadSchema = z.object({
	id: z.string().uuid().optional(),
	body: z.string(),
	post_id: z.string().uuid(),
});

export type CommentCreatePayloadAndUpdatePayload = z.infer<
	typeof CommentCreatePayloadAndUpdatePayloadSchema
>;
export const CommentCreatePayloadSchema =
	CommentCreatePayloadAndUpdatePayloadSchema;
export type CommentCreatePayload = CommentCreatePayloadAndUpdatePayload;
export const CommentUpdatePayloadSchema =
	CommentCreatePayloadAndUpdatePayloadSchema;
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
			label: "Id",
			constraints: {
				required: true,
			},
		},
		{
			name: "organization_id",
			type: "uuid",
			label: "Organization Id",
			constraints: {
				required: true,
			},
		},
		{
			name: "updated_at",
			type: "date-time",
			label: "Updated At",
			constraints: {
				required: true,
			},
		},
		{
			name: "created_at",
			type: "date-time",
			label: "Created At",
			constraints: {
				required: true,
			},
		},
		{
			name: "body",
			type: "text",
			label: "Body",
			constraints: {
				required: true,
			},
		},
		{
			name: "post_id",
			type: "uuid",
			label: "Post Id",
			constraints: {
				required: true,
			},
		},
	],
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
export const OrganizationPopulatedGetSchema = OrganizationSchema;
export type OrganizationPopulatedGet = Organization;
export const OrganizationPopulatedListSchema = OrganizationSchema;
export type OrganizationPopulatedList = Organization;
export const OrganizationCreateResultSchema = OrganizationSchema;
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
export const OrganizationCreatePayloadSchema =
	OrganizationCreatePayloadAndUpdatePayloadSchema;
export type OrganizationCreatePayload =
	OrganizationCreatePayloadAndUpdatePayload;
export const OrganizationUpdatePayloadSchema =
	OrganizationCreatePayloadAndUpdatePayloadSchema;
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
			label: "Id",
			constraints: {
				required: true,
			},
		},
		{
			name: "updated_at",
			type: "date-time",
			label: "Updated At",
			constraints: {
				required: true,
			},
		},
		{
			name: "created_at",
			type: "date-time",
			label: "Created At",
			constraints: {
				required: true,
			},
		},
		{
			name: "name",
			type: "text",
			label: "Name",
			constraints: {
				required: true,
			},
		},
		{
			name: "owner",
			type: "uuid",
			label: "Owner",
			constraints: {
				required: false,
			},
		},
		{
			name: "default_role",
			type: "uuid",
			label: "Default Role",
			constraints: {
				required: false,
			},
		},
		{
			name: "active",
			type: "boolean",
			label: "Active",
			constraints: {
				required: true,
			},
		},
	],
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
export const PollPopulatedGetSchema = PollSchema;
export type PollPopulatedGet = Poll;
export const PollPopulatedListSchema = PollSchema;
export type PollPopulatedList = Poll;
export const PollCreateResultSchema = PollSchema;
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
export const PollCreatePayloadSchema = PollCreatePayloadAndUpdatePayloadSchema;
export type PollCreatePayload = PollCreatePayloadAndUpdatePayload;
export const PollUpdatePayloadSchema = PollCreatePayloadAndUpdatePayloadSchema;
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
			label: "Id",
			constraints: {
				required: true,
			},
		},
		{
			name: "organization_id",
			type: "uuid",
			label: "Organization Id",
			constraints: {
				required: true,
			},
		},
		{
			name: "updated_at",
			type: "date-time",
			label: "Updated At",
			constraints: {
				required: true,
			},
		},
		{
			name: "created_at",
			type: "date-time",
			label: "Created At",
			constraints: {
				required: true,
			},
		},
		{
			name: "question",
			type: "text",
			label: "Question",
			constraints: {
				required: true,
			},
		},
		{
			name: "answers",
			type: "object",
			label: "Answers",
			constraints: {
				required: true,
			},
		},
		{
			name: "post_id",
			type: "uuid",
			label: "Post Id",
			constraints: {
				required: true,
			},
		},
	],
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
export const PostImagePopulatedGetSchema = PostImageSchema;
export type PostImagePopulatedGet = PostImage;
export const PostImagePopulatedListSchema = PostImageSchema;
export type PostImagePopulatedList = PostImage;
export const PostImageCreateResultSchema = PostImageSchema;
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
export const PostImageCreatePayloadSchema =
	PostImageCreatePayloadAndUpdatePayloadSchema;
export type PostImageCreatePayload = PostImageCreatePayloadAndUpdatePayload;
export const PostImageUpdatePayloadSchema =
	PostImageCreatePayloadAndUpdatePayloadSchema;
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
			label: "Id",
			constraints: {
				required: true,
			},
		},
		{
			name: "organization_id",
			type: "uuid",
			label: "Organization Id",
			constraints: {
				required: true,
			},
		},
		{
			name: "updated_at",
			type: "date-time",
			label: "Updated At",
			constraints: {
				required: true,
			},
		},
		{
			name: "created_at",
			type: "date-time",
			label: "Created At",
			constraints: {
				required: true,
			},
		},
		{
			name: "file_storage_key",
			type: "text",
			label: "File Storage Key",
			constraints: {
				required: true,
			},
		},
		{
			name: "file_storage_bucket",
			type: "text",
			label: "File Storage Bucket",
			constraints: {
				required: true,
			},
		},
		{
			name: "file_original_name",
			type: "text",
			label: "File Original Name",
			constraints: {
				required: false,
			},
		},
		{
			name: "file_size",
			type: "integer",
			label: "File Size",
			constraints: {
				required: false,
			},
		},
		{
			name: "file_hash",
			type: "blob",
			label: "File Hash",
			constraints: {
				required: false,
			},
		},
		{
			name: "post_id",
			type: "uuid",
			label: "Post Id",
			constraints: {
				required: true,
			},
		},
	],
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
export const ReactionPopulatedGetSchema = ReactionSchema;
export type ReactionPopulatedGet = Reaction;
export const ReactionPopulatedListSchema = ReactionSchema;
export type ReactionPopulatedList = Reaction;
export const ReactionCreateResultSchema = ReactionSchema;
export type ReactionCreateResult = Reaction;

export const ReactionCreatePayloadAndUpdatePayloadSchema = z.object({
	id: z.string().uuid().optional(),
	type: z.string(),
	post_id: z.string().uuid(),
});

export type ReactionCreatePayloadAndUpdatePayload = z.infer<
	typeof ReactionCreatePayloadAndUpdatePayloadSchema
>;
export const ReactionCreatePayloadSchema =
	ReactionCreatePayloadAndUpdatePayloadSchema;
export type ReactionCreatePayload = ReactionCreatePayloadAndUpdatePayload;
export const ReactionUpdatePayloadSchema =
	ReactionCreatePayloadAndUpdatePayloadSchema;
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
			label: "Id",
			constraints: {
				required: true,
			},
		},
		{
			name: "organization_id",
			type: "uuid",
			label: "Organization Id",
			constraints: {
				required: true,
			},
		},
		{
			name: "updated_at",
			type: "date-time",
			label: "Updated At",
			constraints: {
				required: true,
			},
		},
		{
			name: "created_at",
			type: "date-time",
			label: "Created At",
			constraints: {
				required: true,
			},
		},
		{
			name: "type",
			type: "text",
			label: "Type",
			constraints: {
				required: true,
			},
		},
		{
			name: "post_id",
			type: "uuid",
			label: "Post Id",
			constraints: {
				required: true,
			},
		},
	],
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
export const PostCreateResultSchema = PostSchema;
export type PostCreateResult = Post;

export const PostCreatePayloadAndUpdatePayloadSchema = z.object({
	id: z.string().uuid().optional(),
	subject: z.string(),
	body: z.string(),
});

export type PostCreatePayloadAndUpdatePayload = z.infer<
	typeof PostCreatePayloadAndUpdatePayloadSchema
>;
export const PostCreatePayloadSchema = PostCreatePayloadAndUpdatePayloadSchema;
export type PostCreatePayload = PostCreatePayloadAndUpdatePayload;
export const PostUpdatePayloadSchema = PostCreatePayloadAndUpdatePayloadSchema;
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
			label: "Id",
			constraints: {
				required: true,
			},
		},
		{
			name: "organization_id",
			type: "uuid",
			label: "Organization Id",
			constraints: {
				required: true,
			},
		},
		{
			name: "updated_at",
			type: "date-time",
			label: "Updated At",
			constraints: {
				required: true,
			},
		},
		{
			name: "created_at",
			type: "date-time",
			label: "Created At",
			constraints: {
				required: true,
			},
		},
		{
			name: "subject",
			type: "text",
			label: "Subject",
			description: "A summary of the post",
			constraints: {
				required: true,
			},
		},
		{
			name: "body",
			type: "text",
			label: "Body",
			description: "The text of the post",
			constraints: {
				required: true,
			},
		},
	],
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
export const ReportSectionPopulatedGetSchema = ReportSectionSchema;
export type ReportSectionPopulatedGet = ReportSection;
export const ReportSectionPopulatedListSchema = ReportSectionSchema;
export type ReportSectionPopulatedList = ReportSection;
export const ReportSectionCreateResultSchema = ReportSectionSchema;
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
export const ReportSectionCreatePayloadSchema =
	ReportSectionCreatePayloadAndUpdatePayloadSchema;
export type ReportSectionCreatePayload =
	ReportSectionCreatePayloadAndUpdatePayload;
export const ReportSectionUpdatePayloadSchema =
	ReportSectionCreatePayloadAndUpdatePayloadSchema;
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
			label: "Id",
			constraints: {
				required: true,
			},
		},
		{
			name: "organization_id",
			type: "uuid",
			label: "Organization Id",
			constraints: {
				required: true,
			},
		},
		{
			name: "updated_at",
			type: "date-time",
			label: "Updated At",
			constraints: {
				required: true,
			},
		},
		{
			name: "created_at",
			type: "date-time",
			label: "Created At",
			constraints: {
				required: true,
			},
		},
		{
			name: "name",
			type: "text",
			label: "Name",
			constraints: {
				required: true,
			},
		},
		{
			name: "viz",
			type: "text",
			label: "Viz",
			constraints: {
				required: true,
			},
		},
		{
			name: "options",
			type: "object",
			label: "Options",
			constraints: {
				required: true,
			},
		},
		{
			name: "report_id",
			type: "uuid",
			label: "Report Id",
			constraints: {
				required: true,
			},
		},
	],
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
	report_sections: ReportSectionCreatePayloadSchema.array().optional(),
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
export const ReportPopulatedGetSchema = ReportPopulatedGetAndCreateResultSchema;
export type ReportPopulatedGet = ReportPopulatedGetAndCreateResult;
export const ReportCreateResultSchema = ReportPopulatedGetAndCreateResultSchema;
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
	report_sections: ReportSectionUpdatePayloadSchema.array().optional(),
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
			label: "Id",
			constraints: {
				required: true,
			},
		},
		{
			name: "organization_id",
			type: "uuid",
			label: "Organization Id",
			constraints: {
				required: true,
			},
		},
		{
			name: "updated_at",
			type: "date-time",
			label: "Updated At",
			constraints: {
				required: true,
			},
		},
		{
			name: "created_at",
			type: "date-time",
			label: "Created At",
			constraints: {
				required: true,
			},
		},
		{
			name: "title",
			type: "text",
			label: "Title",
			constraints: {
				required: true,
			},
		},
		{
			name: "description",
			type: "text",
			label: "Description",
			constraints: {
				required: false,
			},
		},
		{
			name: "ui",
			type: "object",
			label: "Ui",
			constraints: {
				required: true,
			},
		},
	],
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
export const RolePopulatedGetSchema = RoleSchema;
export type RolePopulatedGet = Role;
export const RolePopulatedListSchema = RoleSchema;
export type RolePopulatedList = Role;
export const RoleCreateResultSchema = RoleSchema;
export type RoleCreateResult = Role;

export const RoleCreatePayloadAndUpdatePayloadSchema = z.object({
	id: z.string().uuid().optional(),
	name: z.string(),
	description: z.string().optional(),
});

export type RoleCreatePayloadAndUpdatePayload = z.infer<
	typeof RoleCreatePayloadAndUpdatePayloadSchema
>;
export const RoleCreatePayloadSchema = RoleCreatePayloadAndUpdatePayloadSchema;
export type RoleCreatePayload = RoleCreatePayloadAndUpdatePayload;
export const RoleUpdatePayloadSchema = RoleCreatePayloadAndUpdatePayloadSchema;
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
			label: "Id",
			constraints: {
				required: true,
			},
		},
		{
			name: "organization_id",
			type: "uuid",
			label: "Organization Id",
			constraints: {
				required: true,
			},
		},
		{
			name: "updated_at",
			type: "date-time",
			label: "Updated At",
			constraints: {
				required: true,
			},
		},
		{
			name: "created_at",
			type: "date-time",
			label: "Created At",
			constraints: {
				required: true,
			},
		},
		{
			name: "name",
			type: "text",
			label: "Name",
			constraints: {
				required: true,
			},
		},
		{
			name: "description",
			type: "text",
			label: "Description",
			constraints: {
				required: false,
			},
		},
	],
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
export const UserPopulatedGetSchema = UserSchema;
export type UserPopulatedGet = User;
export const UserPopulatedListSchema = UserSchema;
export type UserPopulatedList = User;
export const UserCreateResultSchema = UserSchema;
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
export const UserCreatePayloadSchema = UserCreatePayloadAndUpdatePayloadSchema;
export type UserCreatePayload = UserCreatePayloadAndUpdatePayload;
export const UserUpdatePayloadSchema = UserCreatePayloadAndUpdatePayloadSchema;
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
			label: "Id",
			constraints: {
				required: true,
			},
		},
		{
			name: "organization_id",
			type: "uuid",
			label: "Organization Id",
			constraints: {
				required: false,
			},
		},
		{
			name: "updated_at",
			type: "date-time",
			label: "Updated At",
			constraints: {
				required: true,
			},
		},
		{
			name: "created_at",
			type: "date-time",
			label: "Created At",
			constraints: {
				required: true,
			},
		},
		{
			name: "name",
			type: "text",
			label: "Name",
			constraints: {
				required: true,
			},
		},
		{
			name: "password_hash",
			type: "text",
			label: "Password Hash",
			constraints: {
				required: false,
			},
		},
		{
			name: "email",
			type: "text",
			label: "Email",
			constraints: {
				required: false,
			},
		},
		{
			name: "avatar_url",
			type: "text",
			label: "Avatar Url",
			constraints: {
				required: false,
			},
		},
	],
};
