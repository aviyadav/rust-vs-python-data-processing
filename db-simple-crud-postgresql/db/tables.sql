-- host - localhost
-- port - 5432
-- database - benchmark_poc_db
-- user - pocuser
-- password - password



-- public.ae definition

-- Drop table

-- DROP TABLE public.ae;

CREATE TABLE public.ae (
	"STUDY" varchar NULL,
	"SITE" varchar NULL,
	"SUBJECT" varchar NULL,
	"VISIT" varchar NULL,
	"FORM" varchar NULL,
	"DOMAIN" varchar NULL,
	"AESEQ" int8 NULL,
	"AETERM" varchar NULL,
	"AEDECOD" varchar NULL,
	"AEBODSYS" varchar NULL,
	"AESTDTC" date NULL,
	"AEENDTC" date NULL,
	"AESEV" varchar NULL,
	"AEREL" varchar NULL,
	"AEOUT" varchar NULL,
	"AE_INCIDENT_GROUP" varchar NULL,
	"SITEID" varchar NULL,
	"STUDYID" varchar NULL,
	"USUBJID" varchar NULL
);


-- public.cm definition

-- Drop table

-- DROP TABLE public.cm;

CREATE TABLE public.cm (
	"STUDY" varchar NULL,
	"SITE" varchar NULL,
	"SUBJECT" varchar NULL,
	"VISIT" varchar NULL,
	"FORM" varchar NULL,
	"DOMAIN" varchar NULL,
	"CMSEQ" int8 NULL,
	"CMTRT" varchar NULL,
	"CMDECOD" varchar NULL,
	"CMCAT" varchar NULL,
	"CMSTDTC" date NULL,
	"CMENDTC" date NULL,
	"CMDOSE" float8 NULL,
	"CMDOSU" varchar NULL,
	"CMDOSFRM" varchar NULL,
	"CMROUTE" varchar NULL,
	"CMDOSFRQ" varchar NULL,
	"SITEID" varchar NULL,
	"STUDYID" varchar NULL,
	"USUBJID" varchar NULL
);


-- public.dm definition

-- Drop table

-- DROP TABLE public.dm;

CREATE TABLE public.dm (
	"STUDY" varchar NULL,
	"SITE" varchar NULL,
	"SUBJECT" varchar NULL,
	"VISIT" varchar NULL,
	"FORM" varchar NULL,
	"DOMAIN" varchar NULL,
	"AGE" int8 NULL,
	"SEX" varchar NULL,
	"RACE" varchar NULL,
	"COUNTRY" varchar NULL,
	"DMDTC" date NULL,
	"ARM" varchar NULL,
	"SITEID" varchar NULL,
	"STUDYID" varchar NULL,
	"USUBJID" varchar NULL
);


-- public.lb definition

-- Drop table

-- DROP TABLE public.lb;

CREATE TABLE public.lb (
	"STUDY" varchar NULL,
	"SITE" varchar NULL,
	"SUBJECT" varchar NULL,
	"VISIT" varchar NULL,
	"FORM" varchar NULL,
	"DOMAIN" varchar NULL,
	"LBTESTCD" varchar NULL,
	"LBTEST" varchar NULL,
	"LBORRES" float8 NULL,
	"LBORRESU" varchar NULL,
	"LBSTNRLO" float8 NULL,
	"LBSTNRHI" float8 NULL,
	"LBDTC" date NULL,
	"SITEID" varchar NULL,
	"STUDYID" varchar NULL,
	"USUBJID" varchar NULL
);



-- public.tv definition

-- Drop table

-- DROP TABLE public.tv;

CREATE TABLE public.tv (
	"STUDY" varchar NULL,
	"SITE" varchar NULL,
	"SUBJECT" varchar NULL,
	"VISIT" varchar NULL,
	"FORM" varchar NULL,
	"DOMAIN" varchar NULL,
	"VISITNUM" int8 NULL,
	"TVSTRL" int8 NULL,
	"TVENRL" int8 NULL,
	"ARMCD" varchar NULL,
	"STUDYID" varchar NULL
);

-- public.vs definition

-- Drop table

-- DROP TABLE public.vs;

CREATE TABLE public.vs (
	"STUDY" varchar NULL,
	"SITE" varchar NULL,
	"SUBJECT" varchar NULL,
	"VISIT" varchar NULL,
	"FORM" varchar NULL,
	"DOMAIN" varchar NULL,
	"VSTESTCD" varchar NULL,
	"VSTEST" varchar NULL,
	"VSORRES" float8 NULL,
	"VSORRESU" varchar NULL,
	"VSDTC" date NULL,
	"SITEID" varchar NULL,
	"STUDYID" varchar NULL,
	"USUBJID" varchar NULL
);
