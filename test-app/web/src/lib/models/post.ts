import type { ModelDefinition } from "filigree-web";
import { z } from "zod";
import { ObjectPermission } from "../model_types.js";
import {
	CommentUpdatePayloadSchema,
	CommentSchema,
	CommentCreatePayloadSchema,
} from "./comment.js";
import {
	PollSchema,
	PollCreatePayloadSchema,
	PollUpdatePayloadSchema,
} from "./poll.js";
import {
	PostImageSchema,
	PostImageCreatePayloadSchema,
	PostImageUpdatePayloadSchema,
} from "./post_image.js";
import {
	ReactionUpdatePayloadSchema,
	ReactionSchema,
	ReactionCreatePayloadSchema,
} from "./reaction.js";

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
